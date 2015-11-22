
pub enum Optional<T> {
    Nothing,
    Something(T),
}

fn main() {
    let nothing : Optional<&str> =  Optional::Nothing;
    let something = Optional::Something("hello");

    let vec = vec![nothing, something];

    for thing in vec {
        match thing {
            Optional::Nothing   => println!("Nothing"),
            Optional::Something(value)  => println!("Something: {}", value)
        }
    }

}
// Guaranteed memory safety:
//
// Threads without data races:
//
// Trait-based generics:
//
// Pattern matching:
//
// Type inference:
//
// Minimal runtime:
//
// Efficient C Bindings:
