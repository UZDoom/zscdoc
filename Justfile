debug_run_zforms:
    npm --prefix web_stuff --production run build
    cargo run -- --folder test_data/zforms --output test_data/zforms_docs --delete-without-confirm
    python -m http.server 8080 --bind 127.0.0.1 --directory test_data/zforms_docs

debug_run_gutamatics:
    npm --prefix web_stuff --production run build
    cargo run -- --folder test_data/gutamatics --output test_data/gutamatics_docs --delete-without-confirm
    python -m http.server 8080 --bind 127.0.0.1 --directory test_data/gutamatics_docs

debug_run_uzdoom:
    npm --prefix web_stuff --production run build
    rm -r test_data/uzdoom_docs || true
    mkdir test_data/uzdoom_docs
    cargo run -- --folder ../UZDoom/wadsrc/static/ --output test_data/uzdoom_docs/latest --base-url "/<version>" --delete-without-confirm --target-version latest --versions '[{"url_part": "latest", "nice_name": "4.14.3 (Latest)", "latest": true, "title_suffix": "", "no_index": false}, {"url_part": "v4.14.3", "nice_name": "4.14.3 (Static)", "latest": true, "title_suffix": "4.14.3", "no_index": true}]' --canonical-domain 'https://gutawer.github.io'
    cargo run -- --folder ../UZDoom/wadsrc/static/ --output test_data/uzdoom_docs/v4.14.3 --base-url "/<version>" --delete-without-confirm --target-version v4.14.3 --versions '[{"url_part": "latest", "nice_name": "4.14.3 (Latest)", "latest": true, "title_suffix": "", "no_index": false}, {"url_part": "v4.14.3", "nice_name": "4.14.3 (Static)", "latest": true, "title_suffix": "4.14.3", "no_index": true}]' --canonical-domain 'https://gutawer.github.io'
    python -m http.server 8080 --bind 127.0.0.1 --directory test_data/uzdoom_docs
