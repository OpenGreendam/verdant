//! NumPy's contiguous float64 pairwise reduction order, without temporary
//! arrays for squared/masked signals. This retains the eight-accumulator /
//! 128-element block policy used by the pinned NumPy 2.3.5 ufunc reduction.

pub fn pairwise_sum_iter<I: Iterator<Item = f64>>(input: &mut I, count: usize) -> f64 {
    if count < 8 {
        let mut sum = -0.0;
        for _ in 0..count { sum += input.next().expect("pairwise count matches iterator"); }
        sum
    } else if count <= 128 {
        let mut sums: [f64; 8] = std::array::from_fn(|_| input.next().expect("pairwise count matches iterator"));
        let mut consumed = 8;
        while consumed < count-(count%8) {
            for sum in &mut sums { *sum += input.next().expect("pairwise count matches iterator"); }
            consumed += 8;
        }
        let mut sum = ((sums[0]+sums[1])+(sums[2]+sums[3]))
            + ((sums[4]+sums[5])+(sums[6]+sums[7]));
        for _ in consumed..count { sum += input.next().expect("pairwise count matches iterator"); }
        sum
    } else {
        let split = (count/2) & !7;
        let left = pairwise_sum_iter(input, split);
        let right = pairwise_sum_iter(input, count-split);
        left+right
    }
}

pub fn pairwise_sum(values: &[f64]) -> f64 {
    pairwise_sum_iter(&mut values.iter().copied(), values.len())
}

pub fn voiced_rms(values: &[f64], voiced: &[bool], hop: usize) -> f64 {
    let count = voiced.iter().filter(|&&v| v).count()*hop;
    if count == 0 { return 1.0; }
    let mut squares = values.iter().enumerate()
        .filter(|(index, _)| voiced[*index/hop]).map(|(_,v)| v*v);
    (pairwise_sum_iter(&mut squares, count)/count as f64).sqrt()
}
