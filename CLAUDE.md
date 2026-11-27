<!-- codeguide begin -->
# Change Production Code

Confirmation is needed before a critical action:
+ a command **known or estimated** to be longer than 5 seconds
+ apply `Write/Edit/Bash` tool to edit production source code, i.e. git tracked code in the same folder that conatins **this** CLAUDE.md file

To get such confirmation, present your change skeleton and instructions to run, then literally present 'Say `sanction` to act'. 
Only act on last presented changes as a skeleton if the user says exact word 'sanction' or 'sanctioned'. 
Present markdown for skeleton directly in your response. 
+ Use file name or module path as section header, no need to wrap in `...`
    + The section body is exactly a code block
+ Use `+` for add and `-` for remove **right before target AST item** to indicate changes
    + Show full type/trait definitions. 
    + Fold function body, show function signature and the comment block for that function right before it. 
        + Apply `+` and `-` before the line to show the diff (even if it is not an AST item).  
    + Keep and follow original indentation. 
    + We show it for rust, but it should be the same for other language's AST item 
    ```rust
    fn oldfunc(+a: usize, b: usize) -> Type
    fn oldfunc(-a: usize, b: usize) -> Type
    +fn newfunc(a: usize, b: usize) -> Type
    -fn oldfunc(a: usize, b: usize) -> Type
    ```
    and
    ```rust
    struct A { +newfield: X, -oldfield: Y }
    enum B { +NewVariant, -OldVariant, OldVariantAlt(-X, Y) }
    ```

To inform user about change details, only answer what the user asked for and use a dedicated turn. 

To adjust the skeleton, present only the partial skeleton about files where skeleton adjust in a dedicated turn. 
However, full skeleton must be presented and sanctioned before changes happen. 

To fix an error that happens in a critical action, the fix needs a separate sanctioning. 

# Production Implementation Style

+ **GENERAL**
    + Minimize caller obligation / context dependence:
        + For a given type, can all public methods of an object be called in any order under any concurrency?
        + If cannot, make the type system forbid such usage.
        + If cannot, panic loudly when things go wrong and make sure this obligation does not propagate through the system.
            + Wherever such obligation needs fulfilling, mark them into comment of both callee function and caller function.
        + Caller should be able to use methods without preparation. 
    + Ensure unique path to every ident:
        + To publish a type/function/etc., either re-export through the super module or publish a submodule directly, never both.
    + Tag a function in the last line of its comment block, as `SADDLER: SHAME(MANYARG), ...`, for each limit it exceeds:
        + 6 args `SHAME(MANYARG)`
        + 60 lines for RUST / PYTHON, 100 lines for TYPESCRIPT / CPP `SHAME(TALLFUNC)`
        + 120 chars `SHAME(WIDEFUNC)`
+ **RUST**
    + file / module layout: 
        + `mod` name: one word, or two word `flatcase`
        + either export through `pub use` or `pub mod`, never both
    + module level declaration: 
        + `static` / `const` name should use 1 to 3 word `SNAKE_CAPITAL_CASE`
        + `fn` should use 1 to 3 word `flatcase`
    + type and type class: 
        + `struct` / `enum` / `trait` / `type` name : descriptive `CamelCase` with no word count limit
        + generics type should be abbrieved `CamelCase` like `Ty`
        + `struct` field name : 1 or 2 word `flatcase`; per struct, fields all same length
        + `enum` variant name : 1 or 2 word `CamelCase`; per enum, variants all same length
    + method and variables: 
        + `fn`: 1 or 2 word `flatcase`
        + `let`: 1 or 2 word `flatcase`
    + comment: 
        + always use `/// `, do not comment in function bodies; per block at most 60 words.

# Automated Unit Testing Style

+ **GENERAL**
    + Test from user perspective
        + Please only test what the user will use directly
        + Prefer pseudo random fuzz that utilizes all published methods
        + Test case should be much shorter than implementation
    + Avoid tautology
        + Test written after implementation and passes immediately is garbage
        + Remove them immediately after you find such tests
+ **RUST**
    + `#[cfg(test)] mod correct { ... }` sibling to tested `struct` for correctness
        + `#[test]` + `fn funcname() { ... }`
    + `#[cfg(test)] mod profile { ... }` sibling to tested `struct` for performance
        + `#[test]` + `#[ignore]` + `fn funcname() { ... }`
    + `#[cfg(test)] mod fixture { ... }` for test-only code shared by tests of multiple types or modules

# Git

Commit message like: `feat(scope!): me, a message talking to you`
+ Type: `feat/fix/build/chore/docs/style/refactor/perf/test`
+ Scope (optional): all touched top-level modules
+ Exclaim: breaking API of that module; when breaking almost everything, then `type!: ...`
<!-- codeguide end -->
