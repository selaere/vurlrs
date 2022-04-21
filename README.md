# vurlrs

**vurlrs** is an interpreter and dialect of the [vurl programming language][esolangs] written in rust(🚀🚀) by [me] in 2022, for no reason other than to cure my own boredom. 

it adds some new features, like function scoping, command definitions and function arguments. it adds some commands (that start with underscores `_`), and adds some functionality to some existing commands.

try running `./vurl fizzbuzz.vurl`, or use the (currently very limited and bad) repl by running it without arguments.

[esolangs]: https://esolangs.org/wiki/Vurl
[me]: https://github.com/selaere

## values

vurl has two types of values: _strings_ and _lists_. strings are immutable sequences of unicode characters, and lists are mutable sequences of values. vurl uses the string type for numbers, but vurlrs uses a separate float type. in practice this makes almost no difference, since functions that take numbers will convert strings to numbers, and viceversa. the exception is `eq`, check [comparison commands](#comparison).

## syntax

vurlrs is parsed line by line, and each line can be a _command_, a comment `# ...`, or empty. commands have a command name and arguments, separated by spaces (unless they are in quotes): `add 1 2`

unquoted literals are numbers if they can be converted to numbers, otherwise they are strings. quoted literals are always literals. variable access looks like `[varname]` where _varname_ cannot contain spaces. the results of commands can be used as expressions by using parentheses: `print (add 1 1)`. additionally, [a few commands](#control-flow) use _code blocks_, which are delimited by `end`.

## functions

one thing vurlrs adds is proper functions. vurl has `define` and `call`, which label and run a code block. vurlrs adds local variables, which are relative to the outer `define`. they work exactly like global variables, but their name must start by `.`:

```
define yell_square
    set .a (mul [x] [x])
    # the variable [.a] will not be accessible outside of this function
    print (join [.a] !!!)
end

set x 5
call yell_square
# outputs "25!!!"
```

you can also call these functions with arguments: they will be as a list in the `.args` variable. we can rewrite our function like so:

```
define yell_square
    set .x (index [.args] 1)
    print (join (mul [.x] [.x]) !!!)
end

call yell_square 5
```

there are also return values. you can use the special command `_return`, or you can add a return value to the function's `end`:

```
define compute_yelled_square
    set .x (index [.args] 1)
end (join (mul [.x] [.x]) !!!)

print (call compute_yelled_square 5)
```

there is also an alternative command for defining functions, `_cmd`. it works similar to `define`, but you have to specify a function signature: either a list of named arguments, all of which must start by `.`; or the literal `...`, that doesn't bind any variables (like `define`). its functions don't require `call`, they can be called as commands directly.

```
_cmd compute_yelled_square .x
end (join (mul [.x] [.x]) !!!)

print (compute_yelled_square 5)
```
## commands

### arithmetic

these commands [convert] their arguments to numbers before running.

`add ...` and `mul ...` are variadic, and work with any number of arguments. when provided with no arguments, they return `0` and `1` respectively.

the dyadic commands `sub`, `div`, `mod`, `_pow` and monadic `_exp`, `_floor`, `_round`, `_sqrt`, `_ln`, `_sin`, `_cos`, `_tan`, `_asin`, `_acos`, `_atan` do exactly what you'd expect.

[convert]: https://doc.rust-lang.org/std/primitive.f64.html#method.from_str

### comparison

booleans are the numbers `0` (false) or `1` (true). when a command takes a boolean, it checks if the number is equal to `0.0`, so `NaN 1 -1 3.14 -inf` are all "truthy"

`eq x y` compares numerically when both of its arguments are numbers. this means that `(eq nan nan)` is false (under IEEE-754, NaN is not equal to itself), or `(eq 1.0 1)` is true, but when at least one argument is a string (`(eq nan "nan")` or `(eq inf (substr rainfall 3 5))`) they are compared as strings.

`gt x y`, `gte x y`, `lt x y`, `lte x y` compare two numbers.

`and ...`, `or ...`, `not x` take booleans, and return a boolean. they do no short-circuiting or coalescing.

### strings

lists can be automatically converted to strings, separated by commas and enclosed in parentheses: `print (list a b c (list d e) f)` prints out `(a,b,c,(d,e),f)`.

`join ...` is variadic; and it concatenates multiple values, converting them to strings if necessary. if provided with no arguments it returns an empty string (`""`)

`substr s start stop` returns a substring of _s_, from the index _start_ to _stop_ (inclusive), where _start_ ≤ _stop_. indices start from 1.

`_ord s` returns the unicode codepoint of a single-character string _s_. `_chr n` returns a single-character string with the unicode codepoint _n_.

### lists

`list ...` makes a list with its arguments.

`len x` gets the length of a list, or the length in characters of a string.

indices start from 1. trying to use index 0, or indexing out of range, will error.

`index l i`, `push l v`, `pop l`, `insert l i v`, `remove l i`, `replace l i v` take a reference to _l_ and mutate it.

`_islist x` returns `1` if _x_ is a list, otherwise `0`.

`_clone x` clones the value _x_. for strings (and numbers) this is a noop, but for lists it creates a shallow copy.

### control flow

`while x` and `if x` start code blocks.

`define name` creates a code block that can be called back with `call name [args...]`, and `_cmd name [args...]` defines a command that can be called with just `name [args...]`. note that these must be declared _before_ being used. see [functions](#functions)

`_apply name args` calls command _name_ with the argument list _args_.

`_error x` raises an error with the message _x_.

### variables

`set n v` sets a variable with name _n_. it will be local only if _n_ starts with `.`. it can later be retrieved with `[n]` or `_get n`.

the names of all the locals or globals can be retrieved by calling `_locals` or `_globals` respectively.

### io

`print ...` outputs its arguments to stdout, separated by spaces, with a trailing newline. `_printraw` outputs its arguments to stdout, without separators or newlines. `_printerr` and `_printerrraw` output to stderr instead.

`input` (no arguments) reads one line from stdin.

`_time` gets the current unix time, as seconds.

### random number generation

these commands will only work if the feature `fastrand` is enabled

`_rand` returns a random float between 0 and 1. `_random x y` returns a random integer between x and y, inclusive.
