use ignore::overrides::OverrideBuilder;

pub fn add_documentation_override(mut builder: OverrideBuilder) -> OverrideBuilder {
    // Documentation directories
    builder.add("!**/[Dd]ocs/**").unwrap();
    builder.add("!**/[Dd]oc/**").unwrap();
    builder.add("!**/[Dd]ocumentation/**").unwrap();
    builder.add("!**/[Gg]roovydoc/**").unwrap();
    builder.add("!**/[Jj]avadoc/**").unwrap();
    builder.add("!**/[Mm]an/**").unwrap();
    builder.add("!**/[Ee]xamples/**").unwrap();
    builder.add("!**/[Dd]emo/**").unwrap();
    builder.add("!**/[Dd]emos/**").unwrap();
    builder.add("!**/inst/doc/**").unwrap();

    // Documentation files
    builder.add("!**/CHANGE*").unwrap();
    builder.add("!**/CHANGES*").unwrap();
    builder.add("!**/CHANGELOG*").unwrap();
    builder.add("!**/CONTRIBUTING*").unwrap();
    builder.add("!**/COPYING*").unwrap();
    builder.add("!**/INSTALL*").unwrap();
    builder.add("!**/LICEN[CS]E*").unwrap();
    builder.add("!**/[Ll]icen[cs]e*").unwrap();
    builder.add("!**/README*").unwrap();
    builder.add("!**/[Rr]eadme*").unwrap();

    // Samples folders
    builder.add("!**/[Ss]ample/**").unwrap();
    builder.add("!**/[Ss]amples/**").unwrap();

    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_documentation_directories() {
        let doco = add_documentation_override(OverrideBuilder::new("./"))
            .build()
            .unwrap();
        assert!(doco.matched("/dir/Docs/docfile.ext", false).is_ignore());
        assert!(doco.matched("/dir/Docs/", true).is_ignore());
        assert!(doco.matched("Docs/", true).is_ignore());
        assert!(doco.matched("dir/not-docs/not-doc.ext", false).is_none());
    }

    #[test]
    fn test_documentation_files() {
        let doco = add_documentation_override(OverrideBuilder::new("./"))
            .build()
            .unwrap();
        assert!(doco.matched("/dir/CHANGELOG.md", false).is_ignore());
        assert!(doco.matched("/dir/CHANGELOG", false).is_ignore());
        assert!(doco.matched("/dir/NOT", false).is_none());
    }
}
