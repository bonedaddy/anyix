# AnyIx

`AnyIx` is a lightweight instruction encoding scheme intended to allow arbitrary instruction execution without the need for adding direct support for the instructions within a program. Care needs to be taken when using this as it can easily lead to exploits, to prevent unintended consequences, the reference implementation of the `handle_anyix` function prevents "re-entrant cpi calls".