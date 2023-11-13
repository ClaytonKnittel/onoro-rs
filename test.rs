impl Solution {
  pub fn min_reverse_operations(n: i32, p: i32, banned: Vec<i32>, k: i32) -> Vec<i32> {
    let n = n as usize;
    let p = p as usize;

    let mut ans = vec![-2; n];
    ans[p] = 0;
    banned.into_iter().for_each(|banned_idx| {
      ans[banned_idx as usize] = -1;
    });

    let mut to_explore = Vec::with_capacity(n);
    to_explore.push(p);
    while let Some(idx) = to_explore.pop() {
      let dist = ans[idx];
      let idx = idx as i32;
      let n = n as i32;
      let k = k as i32;
      ((k - idx - 1).abs()..(n - (n - k - idx).abs()))
        .step_by(2)
        .for_each(|new_pos| {
          let a = unsafe { ans.get_unchecked_mut(new_pos as usize) };
          if *a == -2 {
            *a = dist + 1;
            to_explore.push(new_pos as usize);
          }
        });
    }

    // Turn all remaining -2s into -1s
    ans.iter_mut().for_each(|val| {
      *val = (*val).max(-1);
    });

    ans
  }
}
