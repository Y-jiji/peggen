mod json {
    use pigeon::{AstImpl, Num, ParseImpl, Prepend, Space};
    pub enum Json {
        #[rule(r"\[ [0*: {0} , ] \]")]
        Array(RVec<Json>),
        #[rule(r"\{ [0*: {0:`[a-zA-Z][a-zA-Z0-9]*`} : {1} , ] \}")]
        Object(RVec<(String, Json)>),
        #[rule("{0:`[0-9]|[1-9][0-9]+`}")]
        Int(String),
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Json {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Json::Array(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Array",
                        &__self_0,
                    )
                }
                Json::Object(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Object",
                        &__self_0,
                    )
                }
                Json::Int(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Int",
                        &__self_0,
                    )
                }
            }
        }
    }
    impl pigeon::Num for Json {
        fn num(rule: usize) -> usize {
            static DELTA: pigeon::Lazy<usize> = pigeon::Lazy::new(|| {
                pigeon::COUNT.fetch_add(3usize, core::sync::atomic::Ordering::SeqCst)
            });
            *DELTA + rule
        }
    }
    impl<const ERROR: bool> pigeon::ParseImpl<0usize, ERROR> for Json {
        fn parse_impl(
            input: &str,
            end: usize,
            trace: &mut Vec<(usize, usize)>,
            stack: &mut Vec<pigeon::Tag>,
        ) -> Result<usize, ()> {
            pigeon::stacker::maybe_grow(
                32 * 1024,
                1024 * 1024,
                || {
                    let mut last = end;
                    if stack.last().map(|top| top.span.start == end).unwrap_or(false) {
                        return Ok(stack.last().unwrap().span.end);
                    }
                    for &(begin, symb) in trace.iter().rev() {
                        if begin < end {
                            break;
                        }
                        if symb != <Self as pigeon::Num>::num(0usize) {
                            continue;
                        }
                        Err(())?
                    }
                    trace.push((end, <Self as pigeon::Num>::num(0usize)));
                    loop {
                        if let Ok(end) = <Self as pigeon::RuleImpl<
                            0usize,
                            ERROR,
                        >>::rule_impl(input, end, last, trace, stack) {
                            last = end;
                            continue;
                        }
                        if let Ok(end) = <Self as pigeon::RuleImpl<
                            1usize,
                            ERROR,
                        >>::rule_impl(input, end, last, trace, stack) {
                            last = end;
                            continue;
                        }
                        if let Ok(end) = <Self as pigeon::RuleImpl<
                            2usize,
                            ERROR,
                        >>::rule_impl(input, end, last, trace, stack) {
                            last = end;
                            continue;
                        }
                        break;
                    }
                    trace.pop();
                    if last != end { Ok(last) } else { Err(()) }
                },
            )
        }
    }
    impl<const ERROR: bool> pigeon::RuleImpl<0usize, ERROR> for Json
    where
        Self: pigeon::Space,
    {
        #[inline]
        fn rule_impl(
            input: &str,
            end: usize,
            last: usize,
            trace: &mut Vec<(usize, usize)>,
            stack: &mut Vec<pigeon::Tag>,
        ) -> Result<usize, ()> {
            {
                ::std::io::_print(format_args!("{0} :: {1}\n", "Json", "Array"));
            };
            {
                ::std::io::_print(format_args!("\t[start] {0}\n", &input[end..]));
            };
            let size = stack.len();
            let rule = <Self as pigeon::Num>::num(0usize);
            let begin = end;
            let mut inner = || -> Result<usize, ()> {
                let end = if input[end..].starts_with("[") {
                    end + "[".len()
                } else {
                    Err(())?
                };
                let end = Self::space(input, end)?;
                let end = {
                    let begin = end;
                    let mut count = 0usize;
                    let end = loop {
                        let mut end = end;
                        if let Ok(end_) = (|| -> Result<usize, ()> {
                            let end = Self::space(input, end)?;
                            let end = <Json as ParseImpl<
                                0usize,
                                ERROR,
                            >>::parse_impl(input, end, trace, stack)?;
                            let end = Self::space(input, end)?;
                            let end = if input[end..].starts_with(",") {
                                end + ",".len()
                            } else {
                                Err(())?
                            };
                            let end = Self::space(input, end)?;
                            Ok(end)
                        })() {
                            end = end_;
                            count += 1;
                        } else {
                            break end
                        }
                    };
                    stack
                        .push(pigeon::Tag {
                            rule: count,
                            span: begin..end,
                        });
                    end
                };
                let end = Self::space(input, end)?;
                let end = if input[end..].starts_with("]") {
                    end + "]".len()
                } else {
                    Err(())?
                };
                stack
                    .push(pigeon::Tag {
                        rule,
                        span: begin..end,
                    });
                return Ok(end);
            };
            match inner() {
                Ok(end) if end > last => {
                    {
                        ::std::io::_print(format_args!("{0} :: {1}\n", "Json", "Array"));
                    };
                    {
                        ::std::io::_print(
                            format_args!(
                                "\t[ok] {0}\n",
                                &input[begin..end.min(begin + 20)],
                            ),
                        );
                    };
                    {
                        ::std::io::_print(format_args!("\t[stack] {0:?}\n", stack));
                    };
                    Ok(end)
                }
                Err(()) | Ok(..) => {
                    {
                        ::std::io::_print(format_args!("{0} :: {1}\n", "Json", "Array"));
                    };
                    {
                        ::std::io::_print(format_args!("\t[fail] {0}\n", &input[end..]));
                    };
                    {
                        ::std::io::_print(format_args!("\t[stack] {0:?}\n", stack));
                    };
                    stack
                        .resize_with(
                            size,
                            || ::core::panicking::panic(
                                "internal error: entered unreachable code",
                            ),
                        );
                    Err(())
                }
            }
        }
    }
    impl<const ERROR: bool> pigeon::RuleImpl<1usize, ERROR> for Json
    where
        Self: pigeon::Space,
    {
        #[inline]
        fn rule_impl(
            input: &str,
            end: usize,
            last: usize,
            trace: &mut Vec<(usize, usize)>,
            stack: &mut Vec<pigeon::Tag>,
        ) -> Result<usize, ()> {
            {
                ::std::io::_print(format_args!("{0} :: {1}\n", "Json", "Object"));
            };
            {
                ::std::io::_print(format_args!("\t[start] {0}\n", &input[end..]));
            };
            let size = stack.len();
            let rule = <Self as pigeon::Num>::num(1usize);
            let begin = end;
            let mut inner = || -> Result<usize, ()> {
                let end = if input[end..].starts_with("{") {
                    end + "{".len()
                } else {
                    Err(())?
                };
                let end = Self::space(input, end)?;
                let end = {
                    let begin = end;
                    let mut count = 0usize;
                    let end = loop {
                        let mut end = end;
                        if let Ok(end_) = (|| -> Result<usize, ()> {
                            let end = Self::space(input, end)?;
                            let end = {
                                let begin = end;
                                static REGEX: pigeon::Lazy<pigeon::Regex> = pigeon::Lazy::new(||
                                pigeon::Regex::new("^[a-zA-Z][a-zA-Z0-9]*").unwrap());
                                let Some(mat) = REGEX.find(&input[end..]) else { Err(())? };
                                let mat = mat.as_str();
                                let end = end + mat.len();
                                stack
                                    .push(pigeon::Tag {
                                        rule: 0,
                                        span: begin..end,
                                    });
                                end
                            };
                            let end = Self::space(input, end)?;
                            let end = if input[end..].starts_with(":") {
                                end + ":".len()
                            } else {
                                Err(())?
                            };
                            let end = Self::space(input, end)?;
                            let end = <Json as ParseImpl<
                                0usize,
                                ERROR,
                            >>::parse_impl(input, end, trace, stack)?;
                            let end = Self::space(input, end)?;
                            let end = if input[end..].starts_with(",") {
                                end + ",".len()
                            } else {
                                Err(())?
                            };
                            let end = Self::space(input, end)?;
                            Ok(end)
                        })() {
                            end = end_;
                            count += 1;
                        } else {
                            break end
                        }
                    };
                    stack
                        .push(pigeon::Tag {
                            rule: count,
                            span: begin..end,
                        });
                    end
                };
                let end = Self::space(input, end)?;
                let end = if input[end..].starts_with("}") {
                    end + "}".len()
                } else {
                    Err(())?
                };
                stack
                    .push(pigeon::Tag {
                        rule,
                        span: begin..end,
                    });
                return Ok(end);
            };
            match inner() {
                Ok(end) if end > last => {
                    {
                        ::std::io::_print(
                            format_args!("{0} :: {1}\n", "Json", "Object"),
                        );
                    };
                    {
                        ::std::io::_print(
                            format_args!(
                                "\t[ok] {0}\n",
                                &input[begin..end.min(begin + 20)],
                            ),
                        );
                    };
                    {
                        ::std::io::_print(format_args!("\t[stack] {0:?}\n", stack));
                    };
                    Ok(end)
                }
                Err(()) | Ok(..) => {
                    {
                        ::std::io::_print(
                            format_args!("{0} :: {1}\n", "Json", "Object"),
                        );
                    };
                    {
                        ::std::io::_print(format_args!("\t[fail] {0}\n", &input[end..]));
                    };
                    {
                        ::std::io::_print(format_args!("\t[stack] {0:?}\n", stack));
                    };
                    stack
                        .resize_with(
                            size,
                            || ::core::panicking::panic(
                                "internal error: entered unreachable code",
                            ),
                        );
                    Err(())
                }
            }
        }
    }
    impl<const ERROR: bool> pigeon::RuleImpl<2usize, ERROR> for Json
    where
        Self: pigeon::Space,
    {
        #[inline]
        fn rule_impl(
            input: &str,
            end: usize,
            last: usize,
            trace: &mut Vec<(usize, usize)>,
            stack: &mut Vec<pigeon::Tag>,
        ) -> Result<usize, ()> {
            {
                ::std::io::_print(format_args!("{0} :: {1}\n", "Json", "Int"));
            };
            {
                ::std::io::_print(format_args!("\t[start] {0}\n", &input[end..]));
            };
            let size = stack.len();
            let rule = <Self as pigeon::Num>::num(2usize);
            let begin = end;
            let mut inner = || -> Result<usize, ()> {
                let end = {
                    let begin = end;
                    static REGEX: pigeon::Lazy<pigeon::Regex> = pigeon::Lazy::new(|| {
                        pigeon::Regex::new("^[0-9]|[1-9][0-9]+").unwrap()
                    });
                    let Some(mat) = REGEX.find(&input[end..]) else { Err(())? };
                    let mat = mat.as_str();
                    let end = end + mat.len();
                    stack
                        .push(pigeon::Tag {
                            rule: 0,
                            span: begin..end,
                        });
                    end
                };
                stack
                    .push(pigeon::Tag {
                        rule,
                        span: begin..end,
                    });
                return Ok(end);
            };
            match inner() {
                Ok(end) if end > last => {
                    {
                        ::std::io::_print(format_args!("{0} :: {1}\n", "Json", "Int"));
                    };
                    {
                        ::std::io::_print(
                            format_args!(
                                "\t[ok] {0}\n",
                                &input[begin..end.min(begin + 20)],
                            ),
                        );
                    };
                    {
                        ::std::io::_print(format_args!("\t[stack] {0:?}\n", stack));
                    };
                    Ok(end)
                }
                Err(()) | Ok(..) => {
                    {
                        ::std::io::_print(format_args!("{0} :: {1}\n", "Json", "Int"));
                    };
                    {
                        ::std::io::_print(format_args!("\t[fail] {0}\n", &input[end..]));
                    };
                    {
                        ::std::io::_print(format_args!("\t[stack] {0:?}\n", stack));
                    };
                    stack
                        .resize_with(
                            size,
                            || ::core::panicking::panic(
                                "internal error: entered unreachable code",
                            ),
                        );
                    Err(())
                }
            }
        }
    }
    impl<Extra> pigeon::AstImpl<Extra> for Json {
        fn ast<'a>(
            input: &'a str,
            stack: &'a [pigeon::Tag],
            extra: &'a Extra,
        ) -> (&'a [pigeon::Tag], Self) {
            if stack.len() == 0 {
                {
                    ::core::panicking::panic_fmt(format_args!("empty stack"));
                };
            }
            let tag = &stack[stack.len() - 1];
            let stack = &stack[..stack.len() - 1];
            if tag.rule < <Self as pigeon::Num>::num(0) {
                {
                    ::core::panicking::panic_fmt(
                        format_args!("rule number not belong to this type"),
                    );
                };
            }
            if tag.rule >= <Self as pigeon::Num>::num(3usize) {
                {
                    ::core::panicking::panic_fmt(
                        format_args!("rule number not belong to this type"),
                    );
                };
            }
            match tag.rule - <Self as pigeon::Num>::num(0) {
                0usize => {
                    let (stack, _0) = <RVec<
                        Json,
                    > as pigeon::AstImpl<Extra>>::ast(input, stack, extra);
                    (stack, Self::Array(_0))
                }
                1usize => {
                    let (stack, _0) = <RVec<
                        (String, Json),
                    > as pigeon::AstImpl<Extra>>::ast(input, stack, extra);
                    (stack, Self::Object(_0))
                }
                2usize => {
                    let (stack, _0) = {
                        let tag = &stack[stack.len() - 1];
                        (
                            &stack[..stack.len() - 1],
                            <String as core::str::FromStr>::from_str(
                                    &input[tag.span.clone()],
                                )
                                .unwrap(),
                        )
                    };
                    (stack, Self::Int(_0))
                }
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        }
    }
    impl Space for Json {}
    pub struct RVec<T>(Vec<T>);
    #[automatically_derived]
    impl<T: ::core::fmt::Debug> ::core::fmt::Debug for RVec<T> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "RVec", &&self.0)
        }
    }
    impl<T, Extra> AstImpl<Extra> for RVec<T>
    where
        Self: Prepend<Extra>,
        <Self as Prepend<Extra>>::T: AstImpl<Extra>,
    {
        fn ast<'a>(
            input: &'a str,
            stack: &'a [pigeon::Tag],
            extra: &'a Extra,
        ) -> (&'a [pigeon::Tag], Self) {
            let tag = &stack[stack.len() - 1];
            let mut stack = &stack[..stack.len() - 1];
            let mut this = <Self as Prepend<Extra>>::empty();
            for i in 0..tag.rule {
                let (stack, value) = <<Self as Prepend<
                    Extra,
                >>::T as AstImpl<Extra>>::ast(input, stack, extra);
                this.prepend(value, extra);
            }
            (stack, this)
        }
    }
    impl<T, Extra> Prepend<Extra> for RVec<T> {
        type T = T;
        fn empty() -> Self {
            Self(::alloc::vec::Vec::new())
        }
        fn prepend(&mut self, value: Self::T, extra: &Extra) {
            self.0.push(value);
        }
    }
}
