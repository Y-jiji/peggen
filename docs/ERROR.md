# Error Handling

There are two kind of variants: 
- **parse**
    - Caller: called in parse variants
    - Callee: it calls other parse variants
        - if error variants are specified like `Type@{ErrorVariant}`, it is wrong
    - Return: 
        - mismatch before `%` => return `Mismatch(m)`
        - failure before `%` => return `Failure(m)`
        - mismatch/failure after `%`  => return `Failure(m)`
        - in any case a `Fatal` breaks out
- **error** 
    - Caller: when a parse variant return `Failure(m)`, caller is obligated to try adjacent error variants
    - Callee: it calls parse variants
        - in this sense, our error handling is 'shallow'
        - a parse variant called by error variant works just the same
        - it can call error variants if specified
    - Return:
        - both `Mismatch|Failure|Fatal` from callee will be downgraded to `Mismatch(m)`
        - if all error variants `Mismatch`, escalate to `Fatal(m)`
