

/// Macro for reuseable rustdoc that shows an example for isoSPI indexing.
///
/// The diagram is embedded as a base64 data URI since it seems like rustdoc
/// can't treat png paths as literals. If the png ever changes, you can regenerate
/// `isospi_example_diagram.html` from the PNG with:
///
/// ```sh
/// { printf '<img alt="isoSPI two-line chain example" src="data:image/png;base64,'
///   base64 -w0 src/docs/isospi_example_diagram.png
///   printf '">'
/// } > src/docs/isospi_example_diagram.html
/// ```
macro_rules! isospi_indexing_example {
    () => {
        concat!(
            "<details>\n\n",
            "<summary><h4>How to Index (Diagram)</h4></summary>\n\n",
            "Here's a diagram that shows a hypothetical two-line isoSPI configuration with a chain of ADBMS6830B chips:\n\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/docs/isospi_example_diagram.html"
            )),
            "\n\nAs far as this driver is concerned, indexing starts at the closest chip to the host in the `Line`.\n",
            "This means that index `0` would be the first chip in the SPI chain from the POV of that SPI peripheral.\n",
            "You can see this in the diagram: From Line B's POV (blue), the chip at index `0` is the chip at index `9` from Line A's POV (red).\n\n",
            "Note that this indexing convention is specifically for this `adbms6830b` driver and the `Line` struct. Application layers and\n",
            "other higher-level code that use this driver may standardize indexing across multiple lines.\n\n",
            "Also, in case this wasn't clear, the `Line` struct in this driver models a single physical line/chain. So, the application layer for the\n",
            "two-line configuration shown in the diagram would model their setup with two `Line` instances (`line_a` and `line_b`).\n",
            "`line_a` and `line_b` would both be capable of reading the same chips, but their indexing would be flipped from one another\n",
            "(as you can see in the \"Chip x\" labels on each ADBMS6830B chip, where the red labels correspond to that chip's index according to `line_a`,\n",
            "and the blue labels correspond to that chip's index according to `line_b`).\n",
            "\n\n</details>",
        )
    };
}
pub(in crate) use isospi_indexing_example;