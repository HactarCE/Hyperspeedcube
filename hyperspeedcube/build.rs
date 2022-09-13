fn main() {
    #[cfg(all(windows, not(debug_assertions)))]
    {
        // Set application icon.
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon/hyperspeedcube.ico");
        res.compile().unwrap();
    }
}
