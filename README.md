# Pratt-Gen

A recursive descent parser generator library that use inlined macros.

## Json

For example, to parse a json file, you can write the following code, and use `parse<Json>` to parse a json formatted string. That's it.

```rust
use pratt_gen::*;

#[derive(Clone, Copy, ParserImpl, Space)]
pub enum Json<'a> {
    #[parse("{0}")]
    Float(f64),
    #[parse("{0}")]
    Int(i64),
    #[parse("{0}")]
    Str(&'a str),
    #[parse("{{ {0} }}")]
    Obj(&'a Obj<'a>),
    #[parse("[ {0} ]")]
    Arr(&'a Arr<'a>),
}

#[derive(Clone, Copy, ParserImpl, Space)]
pub enum Obj<'a> {
    #[parse("{0} : {1} , {2}")]
    Next(Ident<'a>, Json<'a>, &'a Obj<'a>),
    #[parse("{0} : {1}")]
    Just(Ident<'a>, Json<'a>),
    #[parse("")]
    Null(),
}

#[derive(Clone, Copy, ParserImpl, Space)]
pub enum Arr<'a> {
    #[parse("{0} , {1}")]
    Next(Json<'a>, &'a Arr<'a>),
    #[parse("{0}")]
    Just(Json<'a>),
    #[parse("")]
    Null(),
}
```

The format string is simple. You can think of it as the reverse of a rust printing format string.

```rust
// printing
println!("{0} + {1}", "a", "b");
// when we match {0} to something and {1} to something. 
// we fill it as Add({0}, {1})
#[parse("{0} + {1}", precedence=4)]
Add(&'a Expr<'a>, &'a Expr<'a>),
```

## Precedence

To handle binary expression with left recursion and precedence, you can do this:

```rust
use pratt_gen::*;

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum Expr<'a> {
    #[parse("{0:2} + {1:1}", precedence=2)]
    Add(&'a Expr<'a>, &'a Expr<'a>),
    #[parse("{0:4} + {1:3}", precedence=4)]
    Mul(&'a Expr<'a>, &'a Expr<'a>),
    #[parse("{0}")]
    Number(i64),
}

```

The `precedence=...`, gives a precedence to this pattern, and by putting a number `{...:n}` after a hole, it means we only allow rules with precedence `<n` in that hole.

So `Mul` only allow `Number` on its left hand side.

## Pitfall

A bad thing about recursive descendent parser is that it only supports right association, so `a - b - c` is parsed as `a - (b - c)` , which is not what we normally think it is. But you can write a transformation to remove that or implement add/sub as iterators.
