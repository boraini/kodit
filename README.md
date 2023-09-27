# The Kodit Programming Language

Kodit is intended to be a scripting language that doesn't require knowing a lot of things and instead asks the developer to just write good code with the tooling being unhelpful. I have designed this language in middle school as an experiment, but since I didn't have much programming knowledge back then, things that would make the language general purpose are lacking.

You can run directly using Cargo

```
cargo run -- example-programs/roman-numerals.kdt
```

## Language Structure

Each line is like a UNIX shell command. Each part of the command are separated by whitespace; indentation is also supported although unnecessary. The first word is a command name out of the few available. The rest are arguments, either a string (delimited by quotes), a number (made of digits and punctuation that are used in standard floating point notation), or just a keyword (either to indicate a label or variable). The program runs top-down and control flow control is achieved by jumping to different lines.

## Original Commands

The say command puts the characters in the given string onto the output, or prints the provided number or the value that the provided variable evaluates to.

```
set my_variable 4

say "The answer is "
say my_variable
say 2
```

The label command marks its line as a jump target.

The goto command jumps to the label provided.

```
label start
say "Napoleon\n"
goto start
```

> This was the first program to be run in my 2016 C# implementation for the language.

The ask command prompts the user for some string. Note the use of the special @save variable. @save is used to hold the last operation's i. e. the previous line's output. It works like any other variable other than that.

The set command is used to set a variable. In the original design of the language there is no scoping since there are no methods. In this implementation the variables are scoped between a call and a return command.

```
ask "What is your name? "
say "Hello, "
say @save
say "!\n"

# possible way to enable program exit codes?
set @save 0
```

The sum command can do five arithmetic operations on numbers.

- ```+``` for addition
- ```-``` for subtraction
- ```*``` for multiplication
- ```/``` for division
- ```%``` for division remainder

```
set exponent 5
set result 0

set powered exponent

sum powered / 1
sum result + @save
set result @save

sum powered * exponent
set powered @save

sum powered / 2
sum result + @save
set result @save

...
```


## New Commands

The language needs new commands to make stuff easier to implement and to enable more calculations. A way to have linear data structures makes porting many algorithms from other languages possible.

The if command can jump to two different locations based on a condition. The next label is for convenience and indicates the command should jump to the next line as if nothing has happened.

```
set number -42

sum number > 0
if @save next negative
say "The number is positive."
goto end_sign_check

label negative
say "The number is negative."
label end_sign_check
```

The table command makes a table of m rows and n columns and makes the variable refer to it. We can get a specific row and column and slice specific parts of it. The table will not be copied in case it is given as a function parameter.

```
table my_table 3 4
put my_table 1 1 "Hello"
put my_table 2 3 9

slice my_table 1 1 3 4
get @save 0 0
 # prints Hello
say @save
```

> I was inspired by Lua about this but I might just enable lists as in Lisp or something.

I will improve existing features and add new ones as I feel more confident about the language's design and usefulness.