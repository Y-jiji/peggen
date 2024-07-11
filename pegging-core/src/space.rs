use crate::*;

pub trait Space<'a>: Sized + Copy {
    fn space(source: Source<'a>) -> Result<Source<'a>, Error<'a>> {
        let by = source[..].len() - source[..].trim_start().len();
        Ok(source.proceed(by))
    }
}
