//! Build script that sets the application icon on Windows.

fn main() {
    // Rebuild when locale files change
    println!("cargo::rerun-if-changed=locales");

    hsc_strings::generate_locale_source_code("locales", "src/locales.rs");

    #[cfg(all(windows, not(debug_assertions)))]
    {
        // Set application icon.
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon/hyperspeedcube.ico");
        res.compile().unwrap();
    }
}
