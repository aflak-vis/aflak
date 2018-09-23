use std::io;

use cake;
use primitives;

pub fn save(output: &cake::OutputId, data: &primitives::IOValue) -> io::Result<()> {
    let path = format!("output-{}.fits", output.id());
    println!("Saving output #{} to '{}'", output.id(), &path);
    data.save(&path)?;
    println!("Saved!");
    Ok(())
}
