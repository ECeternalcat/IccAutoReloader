extern crate winres;

fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    res.set_manifest_file("icc_auto_reloader.manifest");
    res.compile().unwrap();
}