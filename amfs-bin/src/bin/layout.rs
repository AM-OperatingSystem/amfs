use amfs::{Superblock, FSGroup};
use type_layout::TypeLayout;
fn main() {
    println!("{}", Superblock::type_layout());
    println!("{}", FSGroup::type_layout());
}
