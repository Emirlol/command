A command parser library in rust. Name is not fully decided yet, may change in the future.

This library is meant for command trees that will be fully defined at compile time.

## How It Works

Every node in the tree is defined using the type system, starting from `Root`:

```rust
let command: Root< // The root node with the convenience methods to execute and inspect the command tree
	Choice< // A choice node, which will try to parse the input with the first child, and if it fails, will try the second child
		Literal< // A literal node that will match a literal string input
			"hello", // The literal string to match
			WithExec< // A node that has both an executor and children, meaning the children of the node in this node are optional and may be omitted (in which case, this WithExec's executor will run)
				Argument< // An Argument node that will parse an argument with a parser type
					"name", // The name of the argument being expected, for help output & general convenience
					StringParser, // The parser type to use for this argument, which will parse the input into a value of the type that the parser returns. Custom argument parsers can be defined by implementing the `ArgParser` trait.
					Exec< // Executor wrapper node to not pollute the type system for functions for any node trait impls
						// The executor function of this node. For executors, the first parameter is always the context for the command tree, which may hold state, mpsc channels, or anything else you want to pass into the executor.
						// The context will be passes in at the root, and will be available on any executor node.
						// The rest of the parameters are the arguments that have been parsed from the parent `Argument` nodes in the execution path, in the order of the nodes in the tree.
						// This means a node has access to the arguments of its parent nodes, but not its children.
						fn(&mut (), String) -> Result<(), Infallible>
					>
				>,
				fn(&mut ()) -> Result<(), Infallible> // The executor of this `WithExec`
			>
		>,
		Choice< // Choice nodes can be chained to create a tree of sibling nodes
			Literal<"hello2", Argument<"name", StringParser, Exec<fn(&mut (), String) -> Result<(), Infallible>>>>,
			Literal<
				"test",
				Argument<
					"arg1",
					StringParser,
					WithExec<
						Argument<"arg2", StringParser, Argument<"arg3", StringParser, Exec<fn(&mut (), String, String, String) -> Result<(), Infallible>>>>,
						Exec<fn(&mut (), String) -> Result<(), Infallible>>,
					>,
				>,
			>,
			BothInspect // Boilerplate to help with the `help` command, to distinguish which children of a `Choice` can be inspected. This is a bit of a leaked abstraction, but it is necessary.
		>,
		BothInspect
	>,
> = command! { /*...*/ }
```

(This is the type for the [basics.rs](./examples/basics.rs) example.)

An explanation of the node types are available in the above snippet.
The only exception is that, strictly speaking, executors are child nodes of the node they are defined in, but they are not part of the tree structure and are not considered when parsing inputs. They are only called when the node is reached during execution.

---

Naturally, this is very verbose and does not help much. The tree is intended to be generated with the `command!` macro, which will create the tree for you and you can store the tree in a variable by relying on type inference.

The type is only needed if you want to use it as a generic parameter, or need to store the command tree as a `const` variable (in which case, you'll also need the `const` feature).

If you *do* need to do one of these, use your IDE to check the type and just copy it from there (and optionally type-alias it to something more suitable), as there doesn't seem to be an intended way to get/build macro outputs' types otherwise.

## `command!`

This is the intended way to define a command tree, and has a mini-DSL.

This macro will return a command tree that stems from a `Root` node, and can be executed with the `.execute` method.
This method has 2 parameters, a context and the string input to parse. The context can be anything, and will be passed into the child nodes' executors during execution. If you don't need to pass any state or context to the child nodes, use `&mut ()` as the type.

There are a few rules to the macro:

- Each scope defines a node. There are two types of nodes: `literal` and `argument`.
  - ```rust
    literal "name" {
        // children
    }

    argument "name": ParserType {
        // children
    }
    ```
- Each scope can optionally add an execution function with the `executes` keyword. This can be a closure expr or a function path. The expected method signature is `(&mut ContextType, {args*}) -> Result<(), {Any error type}>`.
  The args in a node will depend on the parent `argument` nodes in the execution path.
  Nodes that do not have executors can still parse inputs and delegate to the child nodes it has, but will return an `Err(CommandError::IncompleteCommand)` if it's called by itself without any trailing input or children.
  - ```rust
    literal "name" {
        executes |context: &mut ContextType| {
            // ...
        }
    }

    // Alternatively for leaf nodes:
    literal "name" executes |context: &mut ContextType| {
        // ...
    }

    // With the function path:
    literal "name" executes my_function // Where my_function: fn(&mut ContextType) -> Result<(), {Any error type}>
    ```

  - ```rust
    // Arguments are collected in a stack, passed onto the node that has the executor.
     argument "arg1": StringParser {
         executes |context: &mut ContextType, arg1: String| { // arg1 from this node's arguments will be fed to this executor after being parsed
             // ...
         }

         argument "arg2": I32Parser {
             executes |context: &mut ContextType, arg1: String, arg2: i32| { // arg1 string from the parent node will be parsed, and arg2 from this node will also be parsed, both of which will be fed into this executor
                 // ...
             }
         }
     }
    ```
- Nodes may define descriptions for the help page with the `description` keyword. This is optional, and if not defined, `No description provided.` will be printed instead.
  - The description can be any expression that evaluates to a const &'static str. This includes string literals, `concat!` expressions, r#...# raw string literals, names of const variables, etc. Preferably, the description should be a single line.
  - ```rust
    literal "name1" description "This is a description for the help page." {
        // ...
    }

    literal "name2" {
        description {
            concat!("This is a ", "description expression.")
        }
        // ...
    }

    literal "name3" description "This is a description for the help page." executes |context: &mut ContextType| {
        // ...
    }


    argument "arg1": StringParser description "This is a description for the help page." {
        // ...
    }

    argument "arg2": StringParser description "This is a description for the help page." executes |context: &mut ContextType, arg1: String| {
        // ...
    }
    ```
- A node's children may only be all `literal` or all `argument` types. You cannot mix them in the same scope. This is to prevent ambiguity in the command tree during parsing.
- Inputs will be parsed one at a time, in the order that children are defined within the macro (top-to-bottom execution). If a child node fails to parse the input (i.e., it's not the right input for an `argument` node) or doesn't match a literal node, the next child will attempt to parse the same input.


## Examples
Example command tree definitions can be found in [examples](./examples).

## Help
`Root::run` handles the `help` command and returns its formatted help page as `Ok(Some(String))`. For ordinary commands it returns `Ok(None)` after running the executor. `run` does not write to stdout or stderr; executors control their own side effects.

If this kind of auto-help is not desired, you can use `Root::execute` to directly match and execute commands, or `Root::inspect` to get the help page for a specific command without executing it (without the `help` keyword).

```rust
let help = command.inspect("")?;
let command_help = command.inspect("user ban")?;
```

Use the returned `Some(help)` from `run` to print, transmit, or otherwise handle the built-in help response. Executors still return `Result<(), _>` and manage any output or other side effects themselves.

The help page includes a usage line, command descriptions, and positional arguments indented beneath the commands that take them. Enable the opt-in `ansi` feature to add ANSI styling. When enabled, escape codes are included in strings returned by `inspect`.

## Features
- `const`: Enabling this allows the `command!` macro to return a const-assignable value. The command tree evaluation still happens at runtime, but the tree itself can be stored in a `const` variable. The only addition required to make this is to implement (or derive) `ConstDefault` on the relevant `ArgParser` types.
- `argv`: Adds support for argv crate, which allows iterating over the commandline arguments via an `Iterator<Item = &'static OsStr>` implementation. This is mainly for convenience, as it allows you to put in `arv::iter()` as the command input, which will convert the OsStr references into str references for you. This will skip the first argument, which is the executable itself.
- `ansi`: Adds ANSI styling to generated help output. It is disabled by default and adds no dependency.
