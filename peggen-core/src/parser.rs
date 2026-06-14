use core::marker::PhantomData;

use crate::*;

pub struct Parser<T: PeggenTypeStub> {
    ctx: ParseContext,
    _phantom: PhantomData<fn() -> T>,
}

impl<T: PeggenTypeStub> Parser<T> {
    pub fn new() -> Self {
        Self {
            ctx: ParseContext::new(),
            _phantom: PhantomData,
        }
    }

    pub fn parse<'a>(&mut self, input: &str) -> Result<<T as PeggenTypeStub>::Reflect<'a>, ()>
    where <T as PeggenTypeStub>::Reflect<'a>: ParseImpl<0, false> + AstImpl<()>
    {
        self.ctx.clear();
        <<T as PeggenTypeStub>::Reflect<'a> as ParseImpl<0, false>>::parse_impl(input, 0, 0, false, &mut self.ctx)?;
        Ok(<T as PeggenTypeStub>::Reflect::<'a>::peggen_ast(input, &self.ctx.tags, ()).1)
    }

    pub fn parse_with<'a, Extra>(&mut self, input: &str, with: Extra) -> Result<<T as PeggenTypeStub>::Reflect<'a>, ()>
    where <T as PeggenTypeStub>::Reflect<'a>: ParseImpl<0, false> + AstImpl<Extra>,
          Extra: Copy
    {
        self.ctx.clear();
        <<T as PeggenTypeStub>::Reflect<'a> as ParseImpl<0, false>>::parse_impl(input, 0, 0, false, &mut self.ctx)?;
        Ok(<T as PeggenTypeStub>::Reflect::<'a>::peggen_ast(input, &self.ctx.tags, with).1)
    }

    pub fn ref_parse<'a>(&mut self, input: &str) -> Result<<T as PeggenTypeStub>::Reflect<'a>, ()>
    where <T as PeggenTypeStub>::Reflect<'a>: RefParseImpl<0, false> + AstImpl<()>
    {
        self.ctx.clear();
        <<T as PeggenTypeStub>::Reflect<'a> as RefParseImpl<0, false>>::ref_parse_impl(input, 0, 0, false, &mut self.ctx)?;
        Ok(<T as PeggenTypeStub>::Reflect::<'a>::peggen_ast(input, &self.ctx.tags, ()).1)
    }

    pub fn ref_parse_with<'a, Extra>(&mut self, input: &str, with: Extra) -> Result<<T as PeggenTypeStub>::Reflect<'a>, ()>
    where <T as PeggenTypeStub>::Reflect<'a>: RefParseImpl<0, false> + AstImpl<Extra>,
          Extra: Copy
    {
        self.ctx.clear();
        <<T as PeggenTypeStub>::Reflect<'a> as RefParseImpl<0, false>>::ref_parse_impl(input, 0, 0, false, &mut self.ctx)?;
        Ok(<T as PeggenTypeStub>::Reflect::<'a>::peggen_ast(input, &self.ctx.tags, with).1)
    }
}
