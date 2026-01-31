#[cfg(test)]
mod tests {
    use crate::manifest::parse_setup_py;
    use std::time::Instant;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn benchmark_parse_setup_py() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
from setuptools import setup, find_packages

setup(
    name="example-project",
    version="1.2.3",
    packages=find_packages(),
    install_requires=[
        "requests>=2.0.0",
        "numpy>=1.19.0",
        "pandas>=1.1.0",
    ],
    extras_require={{
        "dev": [
            "pytest>=6.0.0",
            "black>=20.8b1",
        ]
    }}
)
"#
        ).unwrap();

        let path = file.path().to_path_buf();

        // Warmup
        for _ in 0..10 {
            let _ = parse_setup_py(&path).await;
        }

        let start = Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let _ = parse_setup_py(&path).await;
        }
        let duration = start.elapsed();

        println!("Time taken for {} iterations: {:?}", iterations, duration);
        println!("Average time per iteration: {:?}", duration / iterations as u32);
    }
}
