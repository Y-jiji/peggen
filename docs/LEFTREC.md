# Left Recursion

We call `Type@{V1, V2, V3}` a **variant group**. 
- Its `u16` index local to `Type` is called `gix`. 
- Combined with the `Type`'s id `tix`, it is called `tgix`

Inside `Matcher`, we use a `track` field to track every `tgix` in from function call. Before we push a new `tgix`, we examine whether there exists the same `tgix` on `track` with the same progress. If so, we treat it as a mismatch and try the next branch. 
