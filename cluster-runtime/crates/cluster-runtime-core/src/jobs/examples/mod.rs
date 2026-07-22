//! Unified example catalog — metadata only; scheduler-specific Python bodies stay in plugin dirs.

pub struct ExampleCatalogEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub packages: &'static [&'static str],
}

static CATALOG: &[ExampleCatalogEntry] = &[
    ExampleCatalogEntry {
        id: "mandelbrot",
        title: "Mandelbrot Renderer",
        description: "Render a Mandelbrot fractal across cluster workers",
        packages: &["numpy"],
    },
    ExampleCatalogEntry {
        id: "monte_carlo_pi",
        title: "Monte Carlo π Estimation",
        description: "Estimate π using random sampling",
        packages: &[],
    },
    ExampleCatalogEntry {
        id: "matrix_multiply",
        title: "Matrix Multiplication",
        description: "Distributed matrix multiply benchmark",
        packages: &["numpy"],
    },
    ExampleCatalogEntry {
        id: "prime_search",
        title: "Prime Number Search",
        description: "Find primes in a range across workers",
        packages: &[],
    },
    ExampleCatalogEntry {
        id: "image_blur",
        title: "Image Blur",
        description: "Apply Gaussian blur to a synthetic image",
        packages: &["numpy"],
    },
    ExampleCatalogEntry {
        id: "word_count",
        title: "Word Count",
        description: "Count words in text lines across workers",
        packages: &[],
    },
];

pub fn get(id: &str) -> Option<&'static ExampleCatalogEntry> {
    CATALOG.iter().find(|e| e.id == id)
}

pub fn list() -> &'static [ExampleCatalogEntry] {
    CATALOG
}

pub fn packages_for(id: &str) -> &'static [&'static str] {
    get(id).map(|e| e.packages).unwrap_or(&[])
}
