debug_run_zforms:
    npm --prefix web_stuff --production run build
    cargo run -- --folder test_data/zforms --output test_data/zforms_docs --delete-without-confirm
    python -m http.server 8080 --bind 127.0.0.1 --directory test_data/zforms_docs

debug_run_gutamatics:
    npm --prefix web_stuff --production run build
    cargo run -- --folder test_data/gutamatics --output test_data/gutamatics_docs --delete-without-confirm
    python -m http.server 8080 --bind 127.0.0.1 --directory test_data/gutamatics_docs
