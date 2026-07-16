A command parser library in rust. Name is not fully decided yet, may change in the future.

This library is meant for command trees that will be fully defined at compile time.

## How It Works

Every node in the tree is defined with a type, starting from `Root`:

```rust
let command: Root<
	Choice<
		Literal<"hello", WithExec<Argument<"name", StringParser, Exec<fn(&mut (), String) -> Result<(), Infallible>>>, Exec<fn(&mut ()) -> Result<(), Infallible>>>>,
		Choice<
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
		>,
	>,
> = command! { /*...*/ }
```

(This is the type for the [basics.rs](./examples/basics.rs) example.)

Naturally, this is very verbose and does not help much. The tree is intended to be generated with the `command!` macro, which will create the tree for you and you can store the tree in a variable by relying on type inference.

The type is only needed if you want to use it as a generic parameter, or need to store the command tree as a `const` variable. 

If you *do* need to do one of these, use your IDE to check the type and just copy it from there (and optionally type-alias it to something more suitable), as there doesn't seem to be an intended way to get/build macro outputs' types otherwise.

## `command!`

This is the intended way to define a command tree.

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
- A node's children may only be all `literal` or all `argument` types. You cannot mix them in the same scope. This is to prevent ambiguity in the command tree during parsing.
- Inputs will be parsed one at a time, in the order that children are defined within the macro (top-to-bottom execution). If a child node fails to parse the input (i.e., it's not the right input for an `argument` node) or doesn't match a literal node, the next child will attempt to parse the same input.


## Examples
Example command tree definitions can be found in [examples](./examples).
