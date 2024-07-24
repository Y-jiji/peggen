
/// ### Brief
/// Eat spaces by default. You may also implement it to ignore comments. 
pub trait Space<'a>: Sized {
    fn space(input: &'a str, begin: usize) -> usize {
        begin + input[begin..].len() - input[begin..].trim_start_matches(|x: char| x.is_whitespace()).len()
    }
}