use crate::*;

#[inline(always)]
pub fn token<'a>(
    source: Source<'a>, 
    err_arena: &'a Arena,
    token: &'static str,
) -> Result<Source<'a>, Error<'a>> {
    let piece = &source[..token.len()];
    if token == piece {
        return Ok(source.proceed(token.len()))
    }
    Err(Error::Mismatch {
        token, 
        range: (source.split, source.split + token.len()), 
        piece: unsafe { err_arena.alloc_str(piece) }
    })
}
