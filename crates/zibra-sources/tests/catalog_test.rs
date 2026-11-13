use zibra_sources::catalog_data::CATALOG_SPECS;
use zibra_sources::LiveSourceRegistry;

#[test]
fn test_catalog_specs_count_and_integrity() {
    assert!(CATALOG_SPECS.len() >= 1600, "Catalog specs must have at least 1600 sources, got {}", CATALOG_SPECS.len());
    
    // Check that every catalog spec has an ID, name, and base URL
    for spec in CATALOG_SPECS {
        assert!(!spec.id.is_empty(), "Source spec must have a non-empty ID");
        assert!(!spec.name.is_empty(), "Source spec must have a non-empty name");
        assert!(spec.base_url.starts_with("http://") || spec.base_url.starts_with("https://"), "Base URL must start with http/https: {}", spec.base_url);
    }
}

#[test]
fn test_registry_initialization() {
    let registry = LiveSourceRegistry::new();
    assert!(registry.total_count() >= 1600, "Registry must contain at least 1600 catalog sources");
    assert!(registry.get("MANGADEX").is_some(), "Flagship MangaDex must exist");
}
