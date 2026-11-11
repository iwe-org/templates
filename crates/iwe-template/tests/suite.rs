mod suite {
    mod archive;
    #[cfg(feature = "cli")]
    mod cli;
    mod disk;
    mod fixture;
    mod indexing;
    mod installing;
    mod reporting;
    mod validation;
}
