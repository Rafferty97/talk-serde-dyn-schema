use ty::{Field, Ty};

mod de;
mod ser;
mod ty;

fn main() {
    let my_struct = struct_def!({
        "name": Ty::String,
        "age": Ty::U64,
        "hobbies": array_def!(Ty::String),
    });

    println!("{:?}", my_struct);
}
