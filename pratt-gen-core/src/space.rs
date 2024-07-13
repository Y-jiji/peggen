use crate::*;

pub trait Space<'a>: Sized + Copy {
    fn space(source: Source<'a>) -> Result<Source<'a>, Error<'a>> {
        let by = source[..].len() - source[..].trim_start_matches(|x: char| x.is_whitespace()).len();
        Ok(source.proceed(by))
    }
}
