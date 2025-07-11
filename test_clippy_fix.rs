// Test file to verify clippy fixes work
#[allow(dead_code)]
fn test_clippy_fix() {
    let x = 5;
    let y = x.clone(); // Clippy should suggest removing .clone() for Copy types
    println!("{}", y);
    
    let vec = vec![1, 2, 3];
    let _len = vec.len(); // Unused variable that clippy can fix
    
    if true == true { // Clippy should suggest simplifying this
        println!("redundant comparison");
    }
}