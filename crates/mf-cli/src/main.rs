use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: mf-cli <input.wmf|emf> <output.svg>");
        return ExitCode::FAILURE;
    }
    let bytes = match std::fs::read(&args[1]) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read error: {e}");
            return ExitCode::FAILURE;
        }
    };
    match metafile::to_svg(&bytes, metafile::RenderOptions::default()) {
        Ok(result) => {
            if let Err(e) = std::fs::write(&args[2], &result.svg) {
                eprintln!("write error: {e}");
                return ExitCode::FAILURE;
            }
            let m = &result.metadata;
            eprintln!(
                "format={:?} bytes={} records={:?} bounds={:?} est={:?}x{:?}",
                m.format, m.byte_length, m.record_count, m.bounds, m.estimated_width, m.estimated_height
            );
            for d in &result.diagnostics {
                eprintln!(
                    "diag: severity={:?} code={} occurrences={} record={:?}({:?}) msg={}",
                    d.severity, d.code, d.occurrences, d.record_name, d.record_index, d.message
                );
            }
            eprintln!("diagnostics total: {}", result.diagnostics.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("render error: {e}");
            ExitCode::FAILURE
        }
    }
}
