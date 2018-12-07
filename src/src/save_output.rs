use cake;
use primitives;

pub fn save(
    output: cake::OutputId,
    data: &primitives::SuccessOut,
) -> Result<(), primitives::ExportError> {
    let path = file_name(data, output);
    println!("Saving output #{} to '{}'", output.id(), &path);
    data.save(&path)?;
    println!("Saved!");
    Ok(())
}

pub fn file_name(data: &primitives::IOValue, output: cake::OutputId) -> String {
    format!("output-{}.{}", output.id(), data.extension())
}
