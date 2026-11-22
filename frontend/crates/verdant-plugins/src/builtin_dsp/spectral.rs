//! Bounded FFT, partitioned FIR and overlap-add spectral processing.
//! Buffers are allocated on construction; redesign and audio processing reuse
//! them. The FFT is a radix-2 Cooley–Tukey transform, independently implemented.
use std::f64::consts::TAU;

#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct Complex { pub re: f64, pub im: f64 }
impl Complex {
    pub fn new(re: f64, im: f64) -> Self { Self { re, im } }
    pub fn mul(self, b: Self) -> Self { Self::new(self.re*b.re-self.im*b.im, self.re*b.im+self.im*b.re) }
    pub fn add(self, b: Self) -> Self { Self::new(self.re+b.re, self.im+b.im) }
    pub fn scale(self, a: f64) -> Self { Self::new(self.re*a, self.im*a) }
    pub fn norm(self) -> f64 { self.re.hypot(self.im) }
}

pub(crate) fn fft(data: &mut [Complex], inverse: bool) {
    let n = data.len();
    debug_assert!(n.is_power_of_two());
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 { j ^= bit; bit >>= 1; }
        j ^= bit;
        if i < j { data.swap(i, j); }
    }
    let mut len = 2;
    while len <= n {
        let angle = if inverse { TAU } else { -TAU } / len as f64;
        let root = Complex::new(angle.cos(), angle.sin());
        for start in (0..n).step_by(len) {
            let mut twiddle = Complex::new(1.0, 0.0);
            for k in 0..len/2 {
                let a = data[start+k];
                let b = data[start+k+len/2].mul(twiddle);
                data[start+k] = a.add(b);
                data[start+k+len/2] = a.add(b.scale(-1.0));
                twiddle = twiddle.mul(root);
            }
        }
        len *= 2;
    }
    if inverse { for x in data { *x = x.scale(1.0/n as f64); } }
}

pub(crate) const FIR_BLOCK: usize = 128;
const FIR_FFT: usize = FIR_BLOCK * 2;
const MAX_TAPS: usize = 4097;
const MAX_PARTS: usize = MAX_TAPS.div_ceil(FIR_BLOCK);
const DESIGN_FFT: usize = 8192;

/// A redesign never replaces the active kernels until all four matrix paths
/// are complete. Each step performs one bounded phase: a caller-selected number
/// of response bins, one 8192-point IFFT, or at most eight 256-point partition
/// FFTs. The pending buffer is allocated alongside the processor, never in audio.
#[derive(Clone,Copy,PartialEq,Eq)]
enum DesignStage { Idle, Samples, Transform, Partitions, Ready }

/// Four kernels form a stereo transfer matrix. Linear-phase designs use a
/// common delay for L/R/M/S combinations and avoid serial latency inflation.
pub(crate) struct PhaseFir {
    kernels: Vec<Complex>,
    pending_kernels: Vec<Complex>,
    history: Vec<Complex>,
    design: Vec<Complex>,
    scratch: Vec<Complex>,
    input: [[f64; FIR_BLOCK]; 2],
    output: [[f64; FIR_BLOCK]; 2],
    overlap: [[f64; FIR_BLOCK]; 2],
    cursor: usize,
    head: usize,
    parts: usize,
    delay: usize,
    paths:[bool;4],
    design_stage:DesignStage,
    design_bin:usize,
    design_path:usize,
    design_part:usize,
    design_taps:usize,
    design_paths:[bool;4],
    #[cfg(test)] pub(crate) designs_started:usize,
    #[cfg(test)] pub(crate) designs_committed:usize,
    #[cfg(test)] pub(crate) work_steps:usize,
}
impl PhaseFir {
    pub fn new() -> Self {
        Self { kernels: vec![Complex::default(); 4*MAX_PARTS*FIR_FFT],
            pending_kernels: vec![Complex::default(); 4*MAX_PARTS*FIR_FFT],
            history: vec![Complex::default(); 2*MAX_PARTS*FIR_FFT],
            design: vec![Complex::default(); DESIGN_FFT*4],
            scratch: vec![Complex::default(); FIR_FFT],
            input: [[0.0; FIR_BLOCK];2], output:[[0.0;FIR_BLOCK];2], overlap:[[0.0;FIR_BLOCK];2],
            cursor:0,head:0,parts:1,delay:0,paths:[false;4],
            design_stage:DesignStage::Idle,design_bin:0,design_path:0,design_part:0,
            design_taps:513,design_paths:[false;4],
            #[cfg(test)] designs_started:0,
            #[cfg(test)] designs_committed:0,
            #[cfg(test)] work_steps:0,
        }
    }
    pub fn reset(&mut self) {
        self.history.fill(Complex::default()); self.input.fill([0.0;FIR_BLOCK]);
        self.output.fill([0.0;FIR_BLOCK]); self.overlap.fill([0.0;FIR_BLOCK]); self.cursor=0; self.head=0;
        self.cancel_design();
    }
    pub fn latency(&self) -> u32 { (FIR_BLOCK+self.delay) as u32 }
    pub fn design_idle(&self)->bool {self.design_stage==DesignStage::Idle}
    pub fn needs_design_step(&self)->bool {matches!(self.design_stage,DesignStage::Samples|DesignStage::Transform|DesignStage::Partitions)}
    pub fn cancel_design(&mut self){self.design_stage=DesignStage::Idle;}
    pub fn begin_design(&mut self,resolution:usize){
        self.design_taps=[513,2049,4097][resolution.min(2)];
        self.design_bin=0;self.design_path=0;self.design_part=0;
        self.design_paths=[false;4];self.design_stage=DesignStage::Samples;
        #[cfg(test)] {self.designs_started+=1;}
    }
    /// Returns true once the pending kernels are complete. New requests must
    /// keep a separate response snapshot until this design finishes, rather
    /// than restarting it on every modulation tick and starving publication.
    pub fn design_step(&mut self,sample_rate:f64,bins:usize,mut response:impl FnMut(f64)->[Complex;4])->bool{
        #[cfg(test)] {if self.needs_design_step(){self.work_steps+=1;}}
        match self.design_stage {
            DesignStage::Idle=>return false,
            DesignStage::Ready=>return true,
            DesignStage::Samples=>{
                let end=(self.design_bin+bins.max(1)).min(DESIGN_FFT/2+1);
                for k in self.design_bin..end {
                    let values=response(k as f64*sample_rate/DESIGN_FFT as f64);
                    for path in 0..4 {
                        let offset=path*DESIGN_FFT;let mut value=values[path];
                        if k==0||k==DESIGN_FFT/2{value.im=0.0;}
                        self.design_paths[path]|=value.norm()>1e-15;
                        self.design[offset+k]=value;
                        if k>0&&k<DESIGN_FFT/2{self.design[offset+DESIGN_FFT-k]=Complex::new(value.re,-value.im);}
                    }
                }
                self.design_bin=end;
                if end==DESIGN_FFT/2+1{self.design_stage=DesignStage::Transform;}
            }
            DesignStage::Transform=>{
                while self.design_path<4&&!self.design_paths[self.design_path]{self.design_path+=1;}
                if self.design_path==4{self.design_stage=DesignStage::Ready;return true;}
                let offset=self.design_path*DESIGN_FFT;
                fft(&mut self.design[offset..offset+DESIGN_FFT],true);
                self.design_part=0;self.design_stage=DesignStage::Partitions;
            }
            DesignStage::Partitions=>{
                let taps=self.design_taps;let half=taps/2;
                let parts=taps.div_ceil(FIR_BLOCK);
                let end=(self.design_part+8).min(parts);
                for part in self.design_part..end {
                    self.scratch.fill(Complex::default());
                    for t in 0..FIR_BLOCK {
                        let index=part*FIR_BLOCK+t;
                        if index<taps {
                            let source=(index+DESIGN_FFT-half)%DESIGN_FFT;
                            let window=0.42-0.5*(TAU*index as f64/(taps-1) as f64).cos()+0.08*(2.0*TAU*index as f64/(taps-1) as f64).cos();
                            self.scratch[t].re=self.design[self.design_path*DESIGN_FFT+source].re*window;
                        }
                    }
                    fft(&mut self.scratch,false);
                    let start=(self.design_path*MAX_PARTS+part)*FIR_FFT;
                    self.pending_kernels[start..start+FIR_FFT].copy_from_slice(&self.scratch);
                }
                self.design_part=end;
                if end==parts {
                    self.design_path+=1;
                    while self.design_path<4&&!self.design_paths[self.design_path]{self.design_path+=1;}
                    self.design_stage=if self.design_path==4{DesignStage::Ready}else{DesignStage::Transform};
                }
            }
        }
        self.design_stage==DesignStage::Ready
    }
    fn finish_design(&mut self){
        debug_assert!(self.design_stage==DesignStage::Ready);
        std::mem::swap(&mut self.kernels,&mut self.pending_kernels);
        self.delay=self.design_taps/2;self.parts=self.design_taps.div_ceil(FIR_BLOCK);
        self.paths=self.design_paths;self.design_stage=DesignStage::Idle;
        #[cfg(test)] {self.designs_committed+=1;}
    }
    /// Publish only between complete convolution blocks. Input FFT history,
    /// overlap and the already queued output remain intact; latency is unchanged.
    pub fn commit_ready(&mut self)->bool{
        if self.design_stage!=DesignStage::Ready||self.cursor!=0{return false;}
        self.finish_design();true
    }
    /// Synchronous setup/control design. Callers use this before audio starts
    /// and for fixed latency changes; ordinary automation uses design_step.
    pub fn design(&mut self,sample_rate:f64,resolution:usize,mut response:impl FnMut(f64)->[f64;4]){
        self.design_complex(sample_rate,resolution,|f|response(f).map(|v|Complex::new(v,0.0)));
    }
    pub fn design_complex(&mut self,sample_rate:f64,resolution:usize,mut response:impl FnMut(f64)->[Complex;4]){
        self.begin_design(resolution);
        while !self.design_step(sample_rate,DESIGN_FFT/2+1,&mut response){}
        self.finish_design();
    }
    pub fn run(&mut self, input:[f64;2])->[f64;2] {
        let result=[self.output[0][self.cursor],self.output[1][self.cursor]];
        for c in 0..2 { self.input[c][self.cursor]=input[c]; }
        self.cursor+=1;
        if self.cursor==FIR_BLOCK {
            self.cursor=0;
            for c in 0..2 {
                self.scratch.fill(Complex::default());
                for i in 0..FIR_BLOCK { self.scratch[i].re=self.input[c][i]; }
                fft(&mut self.scratch,false);
                let start=(c*MAX_PARTS+self.head)*FIR_FFT;
                self.history[start..start+FIR_FFT].copy_from_slice(&self.scratch);
            }
            for c in 0..2 {
                self.scratch.fill(Complex::default());
                for from in 0..2 {
                    let path=c*2+from;
                    if !self.paths[path]{continue;}
                    for part in 0..self.parts {
                        let h=(from*MAX_PARTS+(self.head+MAX_PARTS-part)%MAX_PARTS)*FIR_FFT;
                        let k=(path*MAX_PARTS+part)*FIR_FFT;
                        for bin in 0..FIR_FFT {
                            self.scratch[bin]=self.scratch[bin].add(self.history[h+bin].mul(self.kernels[k+bin]));
                        }
                    }
                }
                fft(&mut self.scratch,true);
                for i in 0..FIR_BLOCK {
                    self.output[c][i]=self.scratch[i].re+self.overlap[c][i];
                    self.overlap[c][i]=self.scratch[FIR_BLOCK+i].re;
                }
            }
            self.head=(self.head+1)%MAX_PARTS;
        }
        result
    }
}

pub(crate) const SPECTRAL_SIZE:usize=2048;
const HOP:usize=SPECTRAL_SIZE/4;
/// Fourfold-overlap Hann analysis/synthesis. Latency is exactly N samples.
/// The callback edits complex L/R bins; external keys remain separate.
pub(crate) struct SpectralFrame {
    ring:Vec<f64>,
    accum:Vec<f64>,
    spectrum:Vec<Complex>,
    window:Vec<f64>,
    cursor:usize,
    hop:usize,
}
impl SpectralFrame {
    pub fn new()->Self {
        Self{ring:vec![0.0;4*SPECTRAL_SIZE],accum:vec![0.0;2*SPECTRAL_SIZE],
            spectrum:vec![Complex::default();4*SPECTRAL_SIZE],
            window:(0..SPECTRAL_SIZE).map(|i|(0.5-0.5*(TAU*i as f64/SPECTRAL_SIZE as f64).cos()).sqrt()).collect(),
            cursor:0,hop:0}
    }
    pub fn reset(&mut self){self.ring.fill(0.0);self.accum.fill(0.0);self.cursor=0;self.hop=0;}
    pub fn run(&mut self,input:[f64;2],key:[f64;2],mut process:impl FnMut(usize,&mut [Complex;2],[Complex;2],&[Complex]))->[f64;2] {
        let c=self.cursor;
        let result=[self.accum[c],self.accum[SPECTRAL_SIZE+c]];
        self.accum[c]=0.0;self.accum[SPECTRAL_SIZE+c]=0.0;
        for channel in 0..4 {self.ring[channel*SPECTRAL_SIZE+c]=if channel<2 {input[channel]} else {key[channel-2]};}
        self.cursor=(c+1)%SPECTRAL_SIZE;
        self.hop+=1;
        if self.hop==HOP {
            self.hop=0;
            for channel in 0..4 {
                for i in 0..SPECTRAL_SIZE {
                    self.spectrum[channel*SPECTRAL_SIZE+i]=Complex::new(self.ring[channel*SPECTRAL_SIZE+(self.cursor+i)%SPECTRAL_SIZE]*self.window[i],0.0);
                }
                fft(&mut self.spectrum[channel*SPECTRAL_SIZE..(channel+1)*SPECTRAL_SIZE],false);
            }
            for bin in 0..=SPECTRAL_SIZE/2 {
                let mut audio=[self.spectrum[bin],self.spectrum[SPECTRAL_SIZE+bin]];
                let key=[self.spectrum[2*SPECTRAL_SIZE+bin],self.spectrum[3*SPECTRAL_SIZE+bin]];
                process(bin,&mut audio,key,&self.spectrum[2*SPECTRAL_SIZE..]);
                for channel in 0..2 {
                    self.spectrum[channel*SPECTRAL_SIZE+bin]=audio[channel];
                    if bin>0 && bin<SPECTRAL_SIZE/2 {
                        self.spectrum[(channel+1)*SPECTRAL_SIZE-bin]=Complex::new(audio[channel].re,-audio[channel].im);
                    }
                }
            }
            for channel in 0..2 {
                fft(&mut self.spectrum[channel*SPECTRAL_SIZE..(channel+1)*SPECTRAL_SIZE],true);
                for i in 0..SPECTRAL_SIZE {
                    self.accum[channel*SPECTRAL_SIZE+(self.cursor+i)%SPECTRAL_SIZE]+=self.spectrum[channel*SPECTRAL_SIZE+i].re*self.window[i]*0.5;
                }
            }
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn fft_roundtrip_and_fir_has_exact_unity_latency() {
        let mut x:Vec<_>=(0..512).map(|i|Complex::new((i as f64*0.17).sin(),0.0)).collect();let original=x.clone();
        fft(&mut x,false);fft(&mut x,true);
        assert!(x.iter().zip(original).all(|(a,b)|(a.re-b.re).abs()<1e-11&&a.im.abs()<1e-11));
        let mut fir=PhaseFir::new();fir.design(48000.0,0,|_|[1.0,0.0,0.0,1.0]);
        let latency=fir.latency() as usize;
        for i in 0..2048 {
            let y=fir.run(if i==0 {[1.0,-0.5]} else {[0.0;2]});
            assert!((y[0]-if i==latency {1.0} else {0.0}).abs()<1e-10,"{i}: {y:?}");
            assert!((y[1]+y[0]*0.5).abs()<1e-12);
        }
    }
    #[test] fn spectral_passthrough_reconstructs_delayed_signal() {
        let mut s=SpectralFrame::new();
        let signal:Vec<_>=(0..12000).map(|i|((i*931%997)as f64/997.0)-0.5).collect();
        for i in 0..signal.len() {
            let y=s.run([signal[i];2],[0.0;2],|_,_,_,_|{});
            let expected=if i>=SPECTRAL_SIZE{signal[i-SPECTRAL_SIZE]}else{0.0};
            assert!((y[0]-expected).abs()<1e-10,"{i}: {} != {expected}",y[0]);
        }
    }
    #[test]
    fn staged_fir_bounds_each_work_slice_and_commits_the_complete_kernel_without_reset(){
        let target=|f:f64|{
            let g=0.7+0.2*(f/24000.0*std::f64::consts::PI).cos();
            [Complex::new(g,0.0),Complex::new(0.1,0.0),Complex::new(-0.05,0.0),Complex::new(g*0.9,0.0)]
        };
        let mut expected=PhaseFir::new();expected.design_complex(48000.0,2,target);
        let mut fir=PhaseFir::new();fir.design(48000.0,2,|_|[1.0,0.0,0.0,1.0]);
        for i in 0..1025{fir.run([(i as f64*0.13).sin();2]);}
        let latency=fir.latency();fir.begin_design(2);
        let mut steps=0;let mut bins=0;
        loop{
            let mut calls=0;
            let ready=fir.design_step(48000.0,64,|f|{calls+=1;target(f)});
            assert!(calls<=64);bins+=calls;steps+=1;
            assert_eq!(fir.latency(),latency);
            if ready{break;}
            for i in 0..32{fir.run([(i as f64*0.23).cos();2]);}
        }
        assert_eq!(bins,4097);assert_eq!(steps,89);
        assert!(!fir.commit_ready(),"a partial input block must retain its current kernel");
        while fir.cursor!=0{fir.run([0.3,-0.2]);}
        let (history,input,output,overlap,head)=(fir.history.clone(),fir.input,fir.output,fir.overlap,fir.head);
        assert!(fir.commit_ready());assert_eq!(fir.latency(),latency);
        assert!(fir.kernels.iter().zip(&expected.kernels).all(|(a,b)|a.re==b.re&&a.im==b.im));
        assert!(fir.history.iter().zip(history).all(|(a,b)|a.re==b.re&&a.im==b.im));
        assert_eq!(fir.input,input);assert_eq!(fir.output,output);assert_eq!(fir.overlap,overlap);assert_eq!(fir.head,head);
        assert_eq!(fir.designs_committed,2);
    }

}
