// eslint-disable-next-line @typescript-eslint/no-unused-vars
import { h } from "tsx-dom";
import * as fuzzysort from "fuzzysort";

type SearchResultKind =
    | "Class"
    | "Struct"
    | "Enum"
    | "Function"
    | "Member"
    | "Constant"
    | "Enumerator";

type SearchResult = {
    name_prelude: string;
    name: string;
    link: string;
    desc: string;
    kind: SearchResultKind;
};

interface SearchResults {
    results: Array<SearchResult>;
}

async function get_search_results(): Promise<SearchResults> {
    const res = await fetch("search.json");
    const res_json = await res.json();
    return res_json as SearchResults;
}
function search(
    text: string,
    search_results: SearchResults,
): Fuzzysort.CancelablePromise<Fuzzysort.KeysResults<SearchResult>> {
    return fuzzysort.goAsync(text, search_results.results, {
        keys: ["name"],
        threshold: -10000,
        limit: 10,
        scoreFn: (e) => {
            // fuzzysort's type declarations appear to be wrong here
            const a = e as unknown as Fuzzysort.KeysResult<SearchResult>;
            if (a[0] == null) return -10001;
            // this very slightly weights the searches towards types rather than things inside types
            return (
                a[0].score +
                {
                    Class: 0.1,
                    Struct: 0.1,
                    Enum: 0.1,
                    Function: 0,
                    Member: 0,
                    Constant: 0,
                    Enumerator: 0,
                }[a.obj.kind]
            );
        },
    });
}

function add_zws(text: string): string {
    for (const c of [".", "_"]) {
        text = text.replace(c, `\u{200B}${c}`);
    }
    return text;
}
function render_search_results(results: Fuzzysort.KeysResults<SearchResult>) {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const search_node = document.getElementById("search")!;
    const search_results_node = document.getElementById("search_results");
    search_results_node?.remove();
    function desc_node(desc: string): HTMLElement {
        const base = <div class="search_text search_desc"></div>;
        base.innerHTML = desc;
        return base;
    }
    if (results.length != 0) {
        const results_slice = results.slice(0, 10);
        const new_search_results = (
            <div id="search_results">
                {results_slice.map((r, i) => (
                    <a href={r.obj.link} class="search_result_link">
                        <div
                            class={`
                                    search_result
                                    ${
                                        i !== results_slice.length - 1
                                            ? " search_result_border"
                                            : ""
                                    }
                                `
                                .trim()
                                .replace(/\s+/g, " ")}
                        >
                            <div class="search_text search_kind">
                                {r.obj.kind.toLowerCase()}
                            </div>
                            <div class="search_text search_name">
                                {add_zws(r.obj.name_prelude)}
                                <span
                                    class={`
                                        ${
                                            {
                                                Class: "class",
                                                Struct: "struct",
                                                Enum: "enum",
                                                Function: "function",
                                                Member: "member",
                                                Constant: "constant",
                                                Enumerator: "constant",
                                            }[r.obj.kind]
                                        }
                                    `}
                                >
                                    {fuzzysort
                                        .highlight(r[0], (m) => (
                                            <span class="highlight_emphasis">
                                                {add_zws(m)}
                                            </span>
                                        ))
                                        ?.map((s) => {
                                            if (s instanceof HTMLElement) {
                                                return s;
                                            } else {
                                                return add_zws(s);
                                            }
                                        })}
                                </span>
                            </div>
                            {desc_node(r.obj.desc)}
                        </div>
                    </a>
                ))}
            </div>
        );
        search_node.appendChild(new_search_results);
    }
}
function remove_search_results() {
    const search_results_node = document.getElementById("search_results");
    search_results_node?.remove();
}

export default async () => {
    const search_results = await get_search_results();

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const search_input = document.getElementById(
        "search_input",
    )! as HTMLInputElement;
    let last_promise: Fuzzysort.CancelablePromise<
        Fuzzysort.KeysResults<SearchResult>
    > | null = null;
    search_input.addEventListener("input", () => {
        last_promise?.cancel();
        last_promise = search(search_input.value, search_results);
        last_promise.then((s) => render_search_results(s));
    });
    search_input.addEventListener("focus", () => {
        last_promise?.cancel();
        last_promise = search(search_input.value, search_results);
        last_promise.then((s) => render_search_results(s));
    });
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const search_node = document.getElementById("search")!;

    search_node.onkeydown = (e) => {
        if (e.key == "Escape") {
            (e.target as HTMLElement).blur();
        }
    };
    search_node.addEventListener("focusout", (ev) => {
        if (
            ev.relatedTarget instanceof Element &&
            search_node.contains(ev.relatedTarget)
        ) {
            return;
        }
        remove_search_results();
    });
};
