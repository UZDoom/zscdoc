import "./fonts.css";
import "./style.scss";

// eslint-disable-next-line @typescript-eslint/no-unused-vars
import { h } from "tsx-dom";
import * as fuzzysort from "fuzzysort";

declare global {
    interface Window {
        toggle_vis: (id: string) => void;
        toggle_sidebar: () => void;
        close_sidebar: () => void;
    }
}
window.toggle_vis = function (id: string) {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const elem = document.getElementById(id)!;
    elem.classList.toggle("hide");
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const button = document.getElementById(id + ".vis_button")!;
    const new_text = elem.classList.contains("hide")
        ? button.innerText.replace("-", "+")
        : button.innerText.replace("+", "-");
    button.innerText = new_text;
};
window.toggle_sidebar = function () {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const elem = document.getElementById("sidebar")!;
    if (elem.style.transform == "translateX(-100%)") {
        elem.style.transform = "translateX(0%)";
    } else {
        elem.style.transform = "translateX(-100%)";
    }
};
let mobile = false;
let sidebar_active = false;

function make_invisible(this: HTMLElement) {
    this.style.visibility = "hidden";
}

window.toggle_sidebar = function () {
    if (!mobile) {
        return;
    }
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const elem = document.getElementById("sidebar")!;
    elem.removeEventListener("transitionend", make_invisible);
    if (!sidebar_active) {
        elem.style.transform = "translateX(0%)";
        elem.style.visibility = "visible";
        sidebar_active = true;
    } else {
        elem.style.transform = "translateX(-100%)";
        elem.addEventListener("transitionend", make_invisible);
        sidebar_active = false;
    }
};
window.close_sidebar = function () {
    if (!mobile) {
        return;
    }
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const elem = document.getElementById("sidebar")!;
    elem.removeEventListener("transitionend", make_invisible);
    elem.style.transform = "translateX(-100%)";
    elem.addEventListener("transitionend", make_invisible);
    sidebar_active = false;
};

const mq = window.matchMedia("(min-width: 481px)");
function size_change(mq: MediaQueryList | MediaQueryListEvent) {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const elem = document.getElementById("sidebar")!;
    if (mq.matches) {
        elem.style.visibility = "";
        elem.style.transform = "";
        mobile = false;
    } else {
        elem.style.visibility = "hidden";
        elem.style.transform = "translateX(-100%)";
        sidebar_active = false;
        mobile = true;
    }
}
mq.addEventListener("change", size_change);

window.addEventListener("DOMContentLoaded", () => {
    {
        const els = document.getElementsByClassName("collapsible");
        for (const el of els) {
            const button_id = el.id + ".vis_button";
            const button = document.getElementById(button_id);
            if (button == null) {
                continue;
            }
            button.style.visibility = "visible";
        }
    }
    {
        const els = document.getElementsByClassName("collapsed_by_default");
        for (const el of els) {
            window.toggle_vis(el.id);
        }
    }
    {
        const els = document.getElementsByClassName("sidebar_clickable");
        for (const el of els) {
            el.addEventListener("click", window.close_sidebar);
        }
    }
    {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const el = document.getElementById("search")!;
        el.style.display = "block";
    }
    {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const el = document.getElementById("header_button")!;
        el.style.display = "block";
    }
    size_change(mq);
});

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
    const ALPHA = Array.from(Array(26)).map((_, i) => i + 65);
    const ALPHABET = ALPHA.map((x) => String.fromCharCode(x));
    for (const c of ALPHABET) {
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

window.onload = async () => {
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
