use std::io;
fn read_number() -> i32 {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().parse().expect("Not a number")
}

fn main() {
    let (x, y): (i32, i32);
    x = read_number();
    y = read_number();
    println!("You entered: {}", x + y);
}
