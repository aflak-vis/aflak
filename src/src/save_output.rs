use cake;
use primitives;

pub fn save(
    output: cake::OutputId,
    data: &primitives::IOValue,
) -> Result<(), primitives::ExportError> {
    let path = file_name(output);
    println!("Saving output #{} to '{}'", output.id(), &path);
    data.save(&path)?;
    println!("Saved!");
    Ok(())
}

pub fn file_name(output: cake::OutputId) -> String {
    format!("output-{}.fits", output.id())
}
