use cake;
use primitives;

pub fn save(
    output: &cake::OutputId,
    data: &primitives::IOValue,
) -> Result<(), primitives::ExportError> {
    let path = format!("output-{}.fits", output.id());
    println!("Saving output #{} to '{}'", output.id(), &path);
    data.save(&path)?;
    println!("Saved!");
    Ok(())
}
