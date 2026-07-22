//! Built-in Ray example jobs — Python bodies live in sibling `.py` files.

pub struct ExampleSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub distributed_body: &'static str,
    pub single_body: Option<&'static str>,
    pub default_args: fn() -> serde_json::Value,
    pub packages: &'static [&'static str],
}

static ALL_EXAMPLES: &[ExampleSpec] = &[
    ExampleSpec {
        id: "mandelbrot",
        title: "Mandelbrot Renderer",
        distributed_body: include_str!("mandelbrot_distributed.py"),
        single_body: Some(include_str!("mandelbrot_single.py")),
        default_args: || serde_json::json!([800, 600, 80]),
        packages: &["numpy"],
    },
    ExampleSpec {
        id: "monte_carlo_pi",
        title: "Monte Carlo π Estimation",
        distributed_body: include_str!("monte_carlo_distributed.py"),
        single_body: Some(include_str!("monte_carlo_single.py")),
        default_args: || serde_json::json!([2_000_000]),
        packages: &[],
    },
    ExampleSpec {
        id: "matrix_multiply",
        title: "Matrix Multiplication",
        distributed_body: include_str!("matrix_distributed.py"),
        single_body: Some(include_str!("matrix_single.py")),
        default_args: || serde_json::json!([256]),
        packages: &["numpy"],
    },
    ExampleSpec {
        id: "prime_search",
        title: "Prime Number Search",
        distributed_body: include_str!("prime_distributed.py"),
        single_body: Some(include_str!("prime_single.py")),
        default_args: || serde_json::json!([50_000]),
        packages: &[],
    },
    ExampleSpec {
        id: "image_blur",
        title: "Image Blur",
        distributed_body: include_str!("blur_distributed.py"),
        single_body: Some(include_str!("blur_single.py")),
        default_args: || serde_json::json!([200, 200]),
        packages: &["numpy"],
    },
    ExampleSpec {
        id: "word_count",
        title: "Word Count",
        distributed_body: include_str!("word_count_distributed.py"),
        single_body: Some(include_str!("word_count_single.py")),
        default_args: || {
            serde_json::json!([[
                "to be or not to be that is the question",
                "whether tis nobler in the mind to suffer",
                "the slings and arrows of outrageous fortune",
                "or to take arms against a sea of troubles"
            ]])
        },
        packages: &[],
    },
];

pub fn get(example_id: &str) -> Option<ResolvedExample<'_>> {
    ALL_EXAMPLES.iter().find(|s| s.id == example_id).map(|spec| ResolvedExample {
        id: spec.id,
        title: spec.title,
        distributed_body: spec.distributed_body,
        single_body: spec.single_body,
        args: (spec.default_args)(),
        packages: spec.packages,
    })
}

pub fn packages_for(example_id: &str) -> &'static [&'static str] {
    ALL_EXAMPLES
        .iter()
        .find(|s| s.id == example_id)
        .map(|s| s.packages)
        .unwrap_or(&[])
}

pub struct ResolvedExample<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub distributed_body: &'a str,
    pub single_body: Option<&'a str>,
    pub args: serde_json::Value,
    pub packages: &'static [&'static str],
}
