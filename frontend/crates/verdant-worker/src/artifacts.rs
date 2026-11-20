//! Rust-owned artifact serialization. Storage dtype/shape and workspace-relative
//! file hashes are the same contracts consumed by the existing cache/verifier.
use std::{fs::{self,File,OpenOptions}, io::{BufReader,BufWriter,Read,Write}, path::{Path,PathBuf}};
use anyhow::{Context,Result,bail,ensure};
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use verdant_synthesis::{Arrays,NumericArray};
use zip::{CompressionMethod,ZipWriter,write::SimpleFileOptions};

pub fn sha(path:&Path)->Result<String>{let mut file=BufReader::new(File::open(path)?);let mut h=Sha256::new();let mut buf=[0;65536];loop{let n=file.read(&mut buf)?;if n==0{break;}h.update(&buf[..n]);}Ok(hex::encode(h.finalize()))}
pub fn write_json(path:&Path,value:&Value)->Result<()>{
    let parent=path.parent().context("JSON parent")?;fs::create_dir_all(parent)?;
    let temp=parent.join(format!(".{}.{}.tmp",path.file_name().unwrap().to_string_lossy(),uuid::Uuid::new_v4()));
    let mut output=BufWriter::new(OpenOptions::new().write(true).create_new(true).open(&temp)?);
    serde_json::to_writer_pretty(&mut output,value)?;output.write_all(b"\n")?;output.flush()?;output.get_ref().sync_all()?;drop(output);
    fs::rename(&temp,path)?;Ok(())
}
pub fn array(shape:Vec<usize>,dtype:&str,values:Vec<f64>)->Result<NumericArray>{NumericArray::new(shape,dtype,values)}
pub fn scalar(value:f64,dtype:&str)->Result<NumericArray>{array(vec![],dtype,vec![value])}
fn dtype(name:&str)->Result<(&'static str,usize)>{Ok(match name{
    "float64"|"f64"|"<f8"|"f8"=>("<f8",8),"float32"|"f32"|"<f4"|"f4"=>("<f4",4),
    "int64"|"i64"|"<i8"|"i8"=>("<i8",8),"int32"|"i32"|"<i4"|"i4"=>("<i4",4),
    "int16"|"i16"|"<i2"|"i2"=>("<i2",2),"uint64"|"u64"|"<u8"|"u8"=>("<u8",8),
    "uint32"|"u32"|"<u4"|"u4"=>("<u4",4),"uint8"|"u8-byte"|"|u1"|"u1"=>("|u1",1),
    "bool"|"bool_"|"|b1"|"b1"=>("|b1",1),_=>bail!("unsupported numerical storage dtype {name}"),
})}
fn write_npy(output:&mut impl Write,array:&NumericArray)->Result<()>{
    let (descr,_)=dtype(&array.dtype)?;
    ensure!(array.shape.iter().try_fold(1usize,|a,&b|a.checked_mul(b))==Some(array.values.len()),"NPY shape does not match values");
    let shape=match array.shape.as_slice(){[]=>"()".into(),[n]=>format!("({n},)"),shape=>format!("({})",shape.iter().map(usize::to_string).collect::<Vec<_>>().join(", "))};
    let mut header=format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': {shape}, }}");
    let padding=(64-(10+header.len()+1)%64)%64;header.extend(std::iter::repeat_n(' ',padding));header.push('\n');
    ensure!(header.len()<=u16::MAX as usize,"NPY header too large");
    output.write_all(b"\x93NUMPY\x01\x00")?;output.write_all(&(header.len() as u16).to_le_bytes())?;output.write_all(header.as_bytes())?;
    for &value in &array.values{
        ensure!(value.is_finite(),"NPY contains nonfinite values");
        match descr{
            "<f8"=>output.write_all(&value.to_le_bytes())?,
            "<f4"=>{let v=value as f32;ensure!(v.is_finite(),"float32 overflow");output.write_all(&v.to_le_bytes())?;},
            "|b1"=>{ensure!(value==0.||value==1.,"boolean array requires 0/1");output.write_all(&[value as u8])?;},
            "<i8"=>{ensure!(value.fract()==0.&&value>=i64::MIN as f64&&value<i64::MAX as f64,"int64 overflow or fractional value");output.write_all(&(value as i64).to_le_bytes())?;},
            "<i4"=>{ensure!(value.fract()==0.&&(i32::MIN as f64..=i32::MAX as f64).contains(&value),"int32 overflow or fractional value");output.write_all(&(value as i32).to_le_bytes())?;},
            "<i2"=>{ensure!(value.fract()==0.&&(i16::MIN as f64..=i16::MAX as f64).contains(&value),"int16 overflow or fractional value");output.write_all(&(value as i16).to_le_bytes())?;},
            "<u8"=>{ensure!(value.fract()==0.&&value>=0.&&value<u64::MAX as f64,"uint64 overflow or fractional value");output.write_all(&(value as u64).to_le_bytes())?;},
            "<u4"=>{ensure!(value.fract()==0.&&(0. ..=u32::MAX as f64).contains(&value),"uint32 overflow or fractional value");output.write_all(&(value as u32).to_le_bytes())?;},
            "|u1"=>{ensure!(value.fract()==0.&&(0. ..=255.).contains(&value),"uint8 overflow or fractional value");output.write_all(&[value as u8])?;},_=>unreachable!(),
        }
    }
    Ok(())
}
pub fn write_npz(path:&Path,arrays:&Arrays)->Result<()>{write_npz_entries(path,arrays.iter().map(|(key,value)|(key.as_str(),value)))}
pub fn write_npz_entries<'a>(path:&Path,arrays:impl IntoIterator<Item=(&'a str,&'a NumericArray)>)->Result<()>{
    let file=BufWriter::new(File::create(path)?);let mut zip=ZipWriter::new(file);
    for (name,array) in arrays{
        ensure!(!name.is_empty()&&!name.contains(['/', '\\'])&&name!="..","invalid NPZ key");
        zip.start_file(format!("{name}.npy"),SimpleFileOptions::default().compression_method(CompressionMethod::Deflated))?;
        write_npy(&mut zip,array)?;
    }
    let mut file=zip.finish()?;file.flush()?;file.get_ref().sync_all()?;Ok(())
}
fn wav_header(out:&mut impl Write,rate:u32,count:usize,bits:u16,format:u16)->Result<()>{
    let size=count.checked_mul(bits as usize/8).context("WAV size overflow")?;
    ensure!(size<=u32::MAX as usize-36,"WAV exceeds RIFF size limit");
    out.write_all(b"RIFF")?;out.write_all(&(36+size as u32).to_le_bytes())?;out.write_all(b"WAVEfmt ")?;
    out.write_all(&16u32.to_le_bytes())?;out.write_all(&format.to_le_bytes())?;out.write_all(&1u16.to_le_bytes())?;out.write_all(&rate.to_le_bytes())?;
    out.write_all(&(rate*(bits as u32/8)).to_le_bytes())?;out.write_all(&(bits/8).to_le_bytes())?;out.write_all(&bits.to_le_bytes())?;
    out.write_all(b"data")?;out.write_all(&(size as u32).to_le_bytes())?;Ok(())
}
pub fn write_pcm16(path:&Path,rate:u32,samples:&[i16])->Result<()>{ensure!(rate>0&&!samples.is_empty(),"WAV requires audio");let mut out=BufWriter::new(File::create(path)?);wav_header(&mut out,rate,samples.len(),16,1)?;for sample in samples{out.write_all(&sample.to_le_bytes())?;}out.flush()?;out.get_ref().sync_all()?;Ok(())}
pub fn write_float32(path:&Path,rate:u32,samples:&[f64])->Result<()>{let mut out=BufWriter::new(File::create(path)?);wav_header(&mut out,rate,samples.len(),32,3)?;for &sample in samples{ensure!(sample.is_finite()&&(sample as f32).is_finite(),"nonfinite WAV");out.write_all(&(sample as f32).to_le_bytes())?;}out.flush()?;Ok(())}
pub fn quantize_native(samples:&[f64])->Result<Vec<i16>>{samples.iter().map(|&x|{ensure!(x.is_finite(),"nonfinite waveform");Ok((x.clamp(-0.999,0.999)*32767.).round_ties_even() as i16)}).collect()}
pub fn pcm16_codes(bytes:&[u8])->Result<(u32,Vec<i16>)>{
    ensure!(bytes.len()>=12&&&bytes[..4]==b"RIFF"&&&bytes[8..12]==b"WAVE","not RIFF/WAVE");
    let(mut position,mut rate,mut pcm)=(12,None,None);
    while position+8<=bytes.len(){
        let tag=&bytes[position..position+4];let size=u32::from_le_bytes(bytes[position+4..position+8].try_into().unwrap()) as usize;
        let start=position+8;let end=start.checked_add(size).filter(|&n|n<=bytes.len()).context("truncated WAV chunk")?;
        if tag==b"fmt "{ensure!(size>=16,"truncated WAV format");let code=u16::from_le_bytes(bytes[start..start+2].try_into().unwrap());let channels=u16::from_le_bytes(bytes[start+2..start+4].try_into().unwrap());let bits=u16::from_le_bytes(bytes[start+14..start+16].try_into().unwrap());ensure!(code==1&&channels==1&&bits==16,"final output requires mono PCM16");rate=Some(u32::from_le_bytes(bytes[start+4..start+8].try_into().unwrap()));}
        if tag==b"data"{ensure!(size%2==0,"PCM16 data alignment");pcm=Some(bytes[start..end].chunks_exact(2).map(|b|i16::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>());}
        position=end+(size&1);
    }
    let(rate,pcm)=(rate.context("missing WAV format")?,pcm.context("missing WAV data")?);ensure!(rate>0&&!pcm.is_empty(),"empty PCM16 audio");Ok((rate,pcm))
}
pub fn metrics(samples:&[f64])->Value{json!({"peak":samples.iter().fold(0.0_f64,|a,&b|a.max(b.abs())),"rms":(samples.iter().map(|v|v*v).sum::<f64>()/samples.len() as f64).sqrt(),"samples":samples.len(),"nonfinite_samples":samples.iter().filter(|x|!x.is_finite()).count(),"clipped_samples":samples.iter().filter(|x|x.abs()>=1.).count()})}
pub fn collect(folder:&Path,workspace:&Path)->Result<Vec<Value>>{
    fn visit(path:&Path,paths:&mut Vec<PathBuf>)->Result<()>{
        let metadata=fs::symlink_metadata(path)?;ensure!(!metadata.file_type().is_symlink(),"job artifact cannot be a symlink");
        if metadata.is_dir(){let mut children=fs::read_dir(path)?.map(|e|e.map(|e|e.path())).collect::<std::io::Result<Vec<_>>>()?;children.sort();for child in children{visit(&child,paths)?;}}
        else if metadata.is_file()&&path.extension().is_some_and(|x|x=="wav"||x=="npz"||x=="json")&&path.file_name().is_some_and(|x|x!="worker-result.json"){paths.push(path.to_owned());}
        Ok(())
    }
    let mut paths=Vec::new();visit(folder,&mut paths)?;paths.sort();
    paths.into_iter().map(|path|Ok(json!({"path":path.strip_prefix(workspace)?.to_string_lossy().replace('\\',"/"),"bytes":fs::metadata(&path)?.len(),"sha256":sha(&path)?}))).collect()
}
