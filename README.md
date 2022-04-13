# vurlrs

**vurlrs** is an interpreter and dialect of the [vurl programming language][esolangs], created by [me] in 2022, for no reason other than to cure my own boredom. 

[esolangs]: https://esolangs.org/wiki/Vurl
[me]: https://github.com/selaere

## values

vurl has two types of values: _strings_ and _lists_. strings are immutable sequences of unicode characters, and _lists_ are mutable sequences of values. vurl uses the string type for arithmetic and numbers, but vurlrs uses a separate type for these. in practice this makes little difference, since functions that take numbers will convert strings to numbers, and viceversa. one exception is `eq`, check [comparison commands](#comparison).

## commands

### arithmetic

these commands [convert] their arguments to numbers before running.

`add` and `mul` are variadic, and work with any number of arguments. when provided with no arguments, they return `0` and `1` respectively.

the dyadic commands `sub`, `div`, `mod`, `_pow` and monadic `_exp`, `_floor`, `_round`, `_sqrt`, `_ln`, `_sin`, `_cos`, `_tan`, `_asin`, `_acos`, `_atan` do exactly what you'd expect.

[convert]: https://doc.rust-lang.org/std/primitive.f64.html#method.from_str

### comparison

booleans are the numbers `0` (false) or `1` (true). when a command takes a boolean, it checks if the number is equal to `0`, so `NaN 1 -1 3.14 -inf` are all "truthy"

`eq x y` compares numerically when both of its arguments are numbers. this means that `(eq nan nan)` is false (under IEEE-754, NaN is not equal to itself), but when at least one argument is a string (`(eq nan "nan")` or `(eq inf (substr rainfall 3 5))`) they are compared as strings, returning true.

`gt`, `gte`, `lt`, `lte` compare two numbers.

`and` and `or` take two booleans, and return another boolean. note that they do no short-circuiting or coalescing.

### strings

lists can be automatically converted to strings, separated by commas and enclosed in parentheses: `print (list a b c (list d e) f)` prints out `(a,b,c,(d,e),f)`.

`join` is variadic; and it concatenates multiple values, converting them to strings if necessary. if provided with no arguments it returns an empty string (`""`)

`substr s start stop` returns a substring of _s_, from the index _start_ to _stop_ (inclusive), where _start_ ≤ _stop_. indices start from 1.

`_ord s` returns the unicode codepoint of a single-character string _s_. `_chr n` returns a single-character string with the unicode codepoint _n_.

### io

`print` outputs its arguments to stdout, separated by spaces, with a trailing newline. `_printraw` outputs its arguments to stdout, without separators or newlines. `_printerr` and `_printerrraw` output to stderr instead.

`input` (no arguments) reads one line from stdin.

### lists

`list ...` makes a list with its arguments.

`len x` gets the length of a list, or the length in characters of a string.

indices start from 0. trying to use index 0, or indexing out of range, will raise an error.

`index l i`, `push l v`, `pop l`, `insert l i v`, `remove l i`, `replace l i v` take a reference to _l_ and mutate it.

`_islist x` returns `1` if _x_ is a list, otherwise `0`.

`_clone x` clones the value _x_. for strings (and numbers) this is a noop, but for lists it creates a [shallow?] copy

### control flow

todo

### randomness

todo

### variables

`set n v` sets a variable with name _n_. it will be local only if _n_ starts with `%`. it can later be retrieved with `[n]` or `_get n`.

...

etc