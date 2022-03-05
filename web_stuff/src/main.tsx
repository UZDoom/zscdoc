import "./fonts.css";
import "./style.scss";

function toggle_vis(elem: HTMLElement, button: HTMLElement) {
    elem.classList.toggle("hide");
    const new_text = elem.classList.contains("hide")
        ? button.innerText.replace("-", "+")
        : button.innerText.replace("+", "-");
    button.innerText = new_text;
}

let mobile = false;
let sidebar_active = false;

function make_invisible(this: HTMLElement) {
    this.style.visibility = "hidden";
}

function toggle_sidebar(sidebar: HTMLElement) {
    if (!mobile) {
        return;
    }
    sidebar.removeEventListener("transitionend", make_invisible);
    if (!sidebar_active) {
        sidebar.style.transform = "translateX(0%)";
        sidebar.style.visibility = "visible";
        sidebar_active = true;
    } else {
        sidebar.style.transform = "translateX(-100%)";
        sidebar.addEventListener("transitionend", make_invisible);
        sidebar_active = false;
    }
}
function close_sidebar(sidebar: HTMLElement) {
    if (!mobile) {
        return;
    }
    sidebar.removeEventListener("transitionend", make_invisible);
    sidebar.style.transform = "translateX(-100%)";
    sidebar.addEventListener("transitionend", make_invisible);
    sidebar_active = false;
}

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
            button.onclick = () => {
                toggle_vis(el as HTMLElement, button);
            };
        }
    }
    {
        const els = document.getElementsByClassName("collapsed_by_default");
        for (const el of els) {
            const button_id = el.id + ".vis_button";
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            const button = document.getElementById(button_id)!;
            toggle_vis(el as HTMLElement, button);
        }
    }
    {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const elem = document.getElementById("sidebar")!;
        const els = document.getElementsByClassName("sidebar_clickable");
        for (const el of els) {
            (el as HTMLElement).onclick = () => {
                close_sidebar(elem);
            };
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
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const elem = document.getElementById("sidebar")!;
        el.onclick = () => {
            toggle_sidebar(elem);
        };
    }
    size_change(mq);

    const search = import(/* webpackChunkName: "search" */ "./search");
    search.then((module) => {
        module.default();
    });
});
