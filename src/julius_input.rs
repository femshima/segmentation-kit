pub fn dfa(num: usize) -> String {
    let mut lines: Vec<String> = (0..num)
        .map(|i| {
            let last_elem = if i == 0 { 1 } else { 0 };
            format!("{} {} {} 0 {}", i, num - i - 1, i + 1, last_elem)
        })
        .collect();
    lines.push(format!("{} -1 -1 1 0", num));
    lines.join("\n")
}

pub fn dict(words: &[&str]) -> String {
    let lines: Vec<String> = words
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{} [w_{}]{}", i, i, s))
        .collect();
    lines.join("\n")
}
