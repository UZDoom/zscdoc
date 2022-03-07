debug_run:
    npm --prefix web_stuff --production run build
    cargo run -- --folder test_data/zforms --output test_data/zforms_docs --delete-without-confirm
    python -m http.server 8080 --bind 127.0.0.1 --directory test_data/zforms_docs
