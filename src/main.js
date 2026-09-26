function tauriInvoke(cmd, args) {
    const g = window.__TAURI__;
    if (g && g.core && typeof g.core.invoke === "function") return g.core.invoke(cmd, args);
    if (g && typeof g.invoke === "function") return g.invoke(cmd, args);
    return Promise.reject(new Error("Tauri API не е достапно."));
}

const state = {
    token: null,
    user: null,
    isAdmin: false,
    isManager: false,
    view: "tables",

    settings: { restaurant_name: "HoReCa", currency: "MKD", vat_rate: "0" },

    categories: [],
    products: [],
    activeCategoryId: null,

    tables: [],

    order: {
        tableId: null,
        tableNumber: null,
        orderId: null,
        items: [],
    },

    kitchenTimer: null,
};

const pinBuffer = { value: "" };

const KIOSK_MODE = new URLSearchParams(window.location.search).get("mode") === "kitchen";
if (KIOSK_MODE) document.documentElement.classList.add("kiosk-mode");

function $(sel, root = document) { return root.querySelector(sel); }
function $all(sel, root = document) { return Array.from(root.querySelectorAll(sel)); }

function escapeHtml(str) {
    return String(str ?? "").replace(/[&<>"']/g, (c) => ({
        "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    }[c]));
}

function toNumber(v) {
    if (v === null || v === undefined) return 0;
    const n = typeof v === "string" ? parseFloat(v) : Number(v);
    return Number.isFinite(n) ? n : 0;
}

function methodLabel(raw) {
    const s = String(raw ?? "");
    if (!s) return "—";
    return s.charAt(0) + s.slice(1).toLowerCase();
}

function money(v) {
    const n = toNumber(v);
    return `${n.toFixed(2)} ${state.settings.currency || ""}`.trim();
}

function toast(message, type = "info") {
    const stack = $("#toast-stack");
    const el = document.createElement("div");
    el.className = `toast toast--${type}`;
    el.textContent = message;
    stack.appendChild(el);
    setTimeout(() => el.remove(), 4200);
}

function errMsg(e) {
    if (typeof e === "string") return e;
    if (e && e.message) return e.message;
    return "Настана непозната грешка.";
}

async function call(cmd, args, opts = {}) {
    try {
        return await tauriInvoke(cmd, args);
    } catch (e) {
        if (!opts.silent) toast(errMsg(e), "error");
        throw e;
    }
}

function openModal(innerHtml) {
    const root = $("#modal-root");
    root.innerHTML = `<button class="modal-close" data-modal-close>&times;</button>${innerHtml}`;
    $("#modal-overlay").hidden = false;
}
function closeModal() {
    $("#modal-overlay").hidden = true;
    $("#modal-root").innerHTML = "";
}
document.addEventListener("click", (e) => {
    if (e.target === $("#modal-overlay") || e.target.closest("[data-modal-close]")) closeModal();
});

$("#win-min").addEventListener("click", () => call("minimize_window", {}, { silent: true }));
$("#win-close").addEventListener("click", () => call("close_window", {}, { silent: true }));
$("#win-max").addEventListener("click", async () => {
    try {
        const maxed = await tauriInvoke("is_maximized", {});
        await tauriInvoke(maxed ? "unmaximize_window" : "maximize_window", {});
    } catch (_) { }
});

function tickClock() {
    const now = new Date();
    $("#sidebar-clock").textContent = now.toLocaleTimeString("mk-MK", { hour: "2-digit", minute: "2-digit" });
}
setInterval(tickClock, 1000 * 20);

function renderPinDots() {
    const dots = $all(".pin-dot");
    dots.forEach((d, i) => d.classList.toggle("is-filled", i < pinBuffer.value.length));
}

$all(".keypad__key").forEach((btn) => {
    btn.addEventListener("click", () => {
        const key = btn.dataset.key;
        if (key === "clear") pinBuffer.value = "";
        else if (key === "back") pinBuffer.value = pinBuffer.value.slice(0, -1);
        else if (pinBuffer.value.length < 6) pinBuffer.value += key;
        renderPinDots();
        $("#login-error").textContent = "";
    });
});

document.addEventListener("keydown", (e) => {
    if (!$("#view-login").classList.contains("is-active")) return;
    if (/^[0-9]$/.test(e.key) && pinBuffer.value.length < 6) { pinBuffer.value += e.key; renderPinDots(); }
    else if (e.key === "Backspace") { pinBuffer.value = pinBuffer.value.slice(0, -1); renderPinDots(); }
    else if (e.key === "Enter") doLogin();
});

$("#btn-login").addEventListener("click", doLogin);

async function doLogin() {
    const pin = pinBuffer.value;
    if (!pin) { $("#login-error").textContent = "Внесете ПИН"; return; }

    const btn = $("#btn-login");
    btn.disabled = true;
    $("#btn-login-label").textContent = "Најавување…";
    $("#login-error").textContent = "";

    try {
        const res = await tauriInvoke("login", { pin });
        state.token = res.token;
        state.user = res;
        state.isAdmin = /admin|админ/i.test(res.role_name || "");
        state.isManager = /admin|админ|manager|менаџер/i.test(res.role_name || "");
        enterApp();
    } catch (e) {
        $("#login-error").textContent = errMsg(e);
        pinBuffer.value = "";
        renderPinDots();
    } finally {
        btn.disabled = false;
        $("#btn-login-label").textContent = "Најави се";
    }
}

function enterApp() {
    pinBuffer.value = "";
    renderPinDots();
    $("#view-login").classList.remove("is-active");
    $("#shell").hidden = false;

    $("#tb-session").hidden = false;
    $("#tb-user-name").textContent = state.user.name || `${state.user.first_name} ${state.user.last_name}`.trim();
    $("#tb-user-role").textContent = state.user.role_name;
    $("#nav-admin-group").style.display = state.isManager ? "" : "none";

    if (KIOSK_MODE) {
        switchView("kitchen");
        return;
    }

    switchView("tables");
    loadTables();
    loadCategories();
}

$("#btn-logout").addEventListener("click", () => {
    state.token = null;
    state.user = null;
    state.isAdmin = false;
    state.isManager = false;
    clearInterval(state.kitchenTimer);
    $("#shell").hidden = true;
    $("#tb-session").hidden = true;
    $("#tb-user-name").textContent = "—";
    $("#tb-user-role").textContent = "—";
    $("#view-login").classList.add("is-active");
});

const ADMIN_VIEWS = new Set(["inventory", "reports", "products", "employees", "settings"]);

function switchView(view) {
    if (ADMIN_VIEWS.has(view) && !state.isManager) return;
    if (KIOSK_MODE && view !== "kitchen") return;
    state.view = view;

    $all(".view-panel").forEach((p) => p.classList.remove("is-active"));
    $all(".nav-item").forEach((n) => n.classList.toggle("is-active", n.dataset.view === view));
    const panel = $(`#panel-${view}`);
    if (panel) panel.classList.add("is-active");

    if (view === "kitchen") { loadKitchen(); startKitchenPolling(); }
    else stopKitchenPolling();

    if (view === "inventory") loadInventory();
    if (view === "reports") loadReports();
    if (view === "products") loadAdminMenu();
    if (view === "employees") loadEmployees();
    if (view === "settings") loadSettings();
    if (view === "tables") loadTables();
}

$all(".nav-item").forEach((btn) => btn.addEventListener("click", () => switchView(btn.dataset.view)));

async function loadSettingsQuiet() {
    try {
        const s = await tauriInvoke("get_settings", {});
        state.settings = s;
        $("#tb-restaurant-name").textContent = s.restaurant_name || "HoReCa";
    } catch (_) { }
}

async function loadTables() {
    if (!state.token) return;
    try {
        state.tables = await call("get_tables", { token: state.token });
        renderTables();
    } catch (_) { }
}

function statusClass(status) {
    if (status === "ЗАФАТЕНА") return "occupied";
    if (status === "РЕЗЕРВИРАНА") return "reserved";
    return "free";
}
function statusLabel(status) {
    if (status === "ЗАФАТЕНА") return "Зафатена";
    if (status === "РЕЗЕРВИРАНА") return "Резервирана";
    return "Слободна";
}

function renderTables() {
    const grid = $("#tables-grid");
    if (!state.tables.length) {
        grid.innerHTML = `<p class="empty-hint">Нема дефинирани маси.</p>`;
        return;
    }
    grid.innerHTML = state.tables.map((t) => {
        const cls = statusClass(t.status);
        const reserveBtn = t.status === "СЛОБОДНА"
            ? `<button class="btn btn--ghost btn--sm" data-action="reserve" title="Резервирај маса">Резервирај</button>`
            : "";
        return `
      <div class="table-card table-card--${cls}" data-table-id="${t.table_id}" data-table-number="${escapeHtml(t.table_number)}" data-status="${escapeHtml(t.status)}" role="button" tabindex="0">
        <span class="table-card__status"></span>
        <div class="table-card__number">Маса ${escapeHtml(t.table_number)}</div>
        <div class="table-card__capacity">${t.capacity} места · ${statusLabel(t.status)}</div>
        ${reserveBtn}
      </div>`;
    }).join("");
}

$("#tables-grid").addEventListener("click", async (e) => {
    const card = e.target.closest(".table-card");
    if (!card) return;
    const tableId = Number(card.dataset.tableId);
    const tableNumber = card.dataset.tableNumber;
    const status = card.dataset.status;

    if (e.target.closest('[data-action="reserve"]')) {
        e.stopPropagation();
        try {
            await call("reserve_table", { token: state.token, tableId });
            toast(`Масата ${tableNumber} е резервирана.`, "success");
            loadTables();
        } catch (_) {}
        return;
    }

    if (status === "РЕЗЕРВИРАНА") {
        openReservedTableModal(tableId, tableNumber);
        return;
    }
    await openTable(tableId, tableNumber, status);
});

function openReservedTableModal(tableId, tableNumber) {
    openModal(`
    <h2>Маса ${escapeHtml(tableNumber)}</h2>
    <p class="modal-sub">Оваа маса е резервирана.</p>
    <div class="modal-actions" style="justify-content:flex-start; flex-wrap:wrap;">
      <button class="btn btn--primary" id="m-open-order">Отвори нарачка</button>
      <button class="btn btn--secondary" id="m-cancel-reservation">Откажи резервација</button>
    </div>
  `);
    $("#m-open-order").addEventListener("click", async () => {
        closeModal();
        await openTable(tableId, tableNumber, "РЕЗЕРВИРАНА");
    });
    $("#m-cancel-reservation").addEventListener("click", async () => {
        try {
            await call("cancel_reservation", { token: state.token, tableId: tableId });
            toast("Резервацијата е откажана.", "success");
            closeModal();
            loadTables();
        } catch (_) {}
    });
}

async function openTable(tableId, tableNumber, status) {
    try {
        let orderId;
        if (status === "ЗАФАТЕНА") {
            orderId = await call("get_active_order", { tableId: tableId });
            if (orderId === null || orderId === undefined) {
                orderId = await call("create_order", { userId: state.user.user_id, tableId: tableId });
            }
        } else {
            orderId = await call("create_order", { userId: state.user.user_id, tableId: tableId });
        }

        state.order.tableId = tableId;
        state.order.tableNumber = tableNumber;
        state.order.orderId = orderId;

        $("#order-table-title").textContent = `Маса ${tableNumber}`;
        switchView("order");
        await Promise.all([loadCategories(), loadCartItems()]);
        renderCategoryTabs();
        renderProductGrid();
    } catch (_) {}
}

$("#btn-back-to-tables").addEventListener("click", () => switchView("tables"));

async function loadCategories() {
    try {
        state.categories = await call("get_categories", {}, { silent: true });
    } catch (_) { state.categories = []; }
}

async function loadAllProducts() {
    try {
        state.products = await call("get_products", { token: state.token });
    } catch (_) { state.products = []; }
}

function renderCategoryTabs() {
    const wrap = $("#category-tabs");
    const tabs = [`<button class="category-tab ${state.activeCategoryId === null ? "is-active" : ""}" data-cat="all">Сите</button>`]
        .concat(state.categories.map((c) => `
      <button class="category-tab ${state.activeCategoryId === c.category_id ? "is-active" : ""}" data-cat="${c.category_id}">${escapeHtml(c.name)}</button>
    `));
    wrap.innerHTML = tabs.join("");
}

$("#category-tabs").addEventListener("click", async (e) => {
    const btn = e.target.closest(".category-tab");
    if (!btn) return;
    state.activeCategoryId = btn.dataset.cat === "all" ? null : Number(btn.dataset.cat);
    renderCategoryTabs();
    await renderProductGrid();
});

async function renderProductGrid() {
    const grid = $("#product-grid");
    grid.innerHTML = `<p class="empty-hint">Вчитување…</p>`;
    try {
        const products = state.activeCategoryId === null
            ? await call("get_products", { token: state.token }, { silent: true })
            : await call("get_products_by_category", { token: state.token, categoryId: state.activeCategoryId }, { silent: true });

        if (!products.length) {
            grid.innerHTML = `<p class="empty-hint">Нема производи во оваа категорија.</p>`;
            return;
        }
        grid.innerHTML = products.map((p) => `
      <button class="product-card" data-product-id="${p.product_id}">
        <span class="product-card__name">${escapeHtml(p.name)}</span>
        <span class="product-card__desc">${escapeHtml(p.description || "")}</span>
        <span class="product-card__price">${money(p.price)}</span>
      </button>
    `).join("");
    } catch (_) {
        grid.innerHTML = `<p class="empty-hint">Грешка при вчитување.</p>`;
    }
}

$("#product-grid").addEventListener("click", async (e) => {
    const card = e.target.closest(".product-card");
    if (!card || !state.order.orderId) return;
    const productId = Number(card.dataset.productId);
    try {
        await call("add_order_item", {
            token: state.token, orderId: state.order.orderId, productId, quantity: 1,
        });
        await loadCartItems();
    } catch (_) {}
});

async function loadCartItems() {
    if (!state.order.orderId) return;
    try {
        state.order.items = await call("get_order_items", { token: state.token, orderId: state.order.orderId }, { silent: true });
        renderCart();
    } catch (_) {}
}

function renderCart() {
    const wrap = $("#cart-items");
    const items = state.order.items;
    if (!items.length) {
        wrap.innerHTML = `<p class="empty-hint">Нема ставки во нарачката</p>`;
    } else {
        wrap.innerHTML = items.map((it) => `
      <div class="cart-item" data-product-id="${it.product_id}">
        <div class="cart-item__info">
          <div class="cart-item__name">${escapeHtml(it.product_name)}</div>
          <div class="cart-item__price">${money(it.unit_price)} / ед.</div>
        </div>
        <div class="cart-item__qty">
          <button class="qty-btn" data-action="dec">−</button>
          <span class="cart-item__qty-val">${it.quantity}</span>
          <button class="qty-btn" data-action="inc">+</button>
        </div>
        <button class="cart-item__remove" data-action="remove" title="Отстрани">✕</button>
      </div>
    `).join("");
    }

    const total = items.reduce((sum, it) => sum + toNumber(it.quantity) * toNumber(it.unit_price), 0);
    $("#cart-total").textContent = money(total);
    $("#order-status-chip").textContent = items.length ? "Активна нарачка" : "Нова нарачка";
}

$("#cart-items").addEventListener("click", async (e) => {
    const row = e.target.closest(".cart-item");
    if (!row) return;
    const action = e.target.closest("[data-action]")?.dataset.action;
    if (!action) return;
    const productId = Number(row.dataset.productId);
    const item = state.order.items.find((i) => i.product_id === productId);
    if (!item) return;

    try {
        if (action === "inc") {
            await call("update_order_quantity", { orderId: state.order.orderId, productId, quantity: item.quantity + 1 });
        } else if (action === "dec") {
            if (item.quantity <= 1) {
                await call("delete_order_item", { orderId: state.order.orderId, productId });
            } else {
                await call("update_order_quantity", { orderId: state.order.orderId, productId, quantity: item.quantity - 1 });
            }
        } else if (action === "remove") {
            await call("delete_order_item", { orderId: state.order.orderId, productId });
        }
        await loadCartItems();
    } catch (_) {}
});

$("#btn-pay").addEventListener("click", () => {
    if (!state.order.items.length) { toast("Нарачката е празна.", "error"); return; }
    let selected = "КЕШ";
    openModal(`
    <h2>Наплата — Маса ${escapeHtml(state.order.tableNumber)}</h2>
    <p class="modal-sub">Вкупен износ: <strong>${money(state.order.items.reduce((s, i) => s + toNumber(i.quantity) * toNumber(i.unit_price), 0))}</strong></p>
    <div class="pay-methods">
      <button class="pay-method is-selected" data-method="КЕШ">Кеш</button>
      <button class="pay-method" data-method="КАРТИЧКА">Картичка</button>
    </div>
    <div class="modal-actions">
      <button class="btn btn--secondary" data-modal-close>Откажи</button>
      <button class="btn btn--primary" id="m-confirm-pay">Потврди наплата</button>
    </div>
  `);
    $all(".pay-method").forEach((b) => b.addEventListener("click", () => {
        $all(".pay-method").forEach((x) => x.classList.remove("is-selected"));
        b.classList.add("is-selected");
        selected = b.dataset.method;
    }));
    $("#m-confirm-pay").addEventListener("click", async () => {
        try {
            const invoiceNumber = await call("pay_order", { orderId: state.order.orderId, method: selected });
            toast("Нарачката е наплатена.", "success");
            await showInvoice(invoiceNumber);
            state.order = { tableId: null, tableNumber: null, orderId: null, items: [] };
            loadTables();
        } catch (_) {}
    });
});

async function showInvoice(invoiceNumber) {
    try {
        const inv = await call("get_invoice", { token: state.token, invoiceNumber });
        const itemsHtml = inv.items.map((it) => `
      <div class="receipt__line">
        <span>${escapeHtml(it.name)} × ${it.quantity}</span>
        <span>${money(toNumber(it.unit_price) * toNumber(it.quantity))}</span>
      </div>
    `).join("");
        openModal(`
      <h2>Фактура издадена</h2>
      <div class="receipt">
        <div class="receipt__title">${escapeHtml(inv.restaurant_name)}</div>
        <div class="receipt__meta">${escapeHtml(inv.invoice_number)} · ${new Date(inv.issued_at).toLocaleString("mk-MK")}</div>
        <div class="receipt__rule"></div>
        ${itemsHtml}
        <div class="receipt__rule"></div>
        <div class="receipt__line"><span>Начин на плаќање</span><span>${escapeHtml(methodLabel(inv.method))}</span></div>
        <div class="receipt__line"><span>ДДВ</span><span>${escapeHtml(inv.vat_rate)}%</span></div>
        <div class="receipt__rule"></div>
        <div class="receipt__total"><span>Вкупно</span><span>${inv.total_amount} ${escapeHtml(inv.currency)}</span></div>
      </div>
      <div class="modal-actions">
        <button class="btn btn--secondary" id="m-print-invoice">Печати</button>
        <button class="btn btn--primary" data-modal-close>Готово</button>
      </div>
    `);
        $("#m-print-invoice").addEventListener("click", () => window.print());
    } catch (_) {
        closeModal();
    }
}

function startKitchenPolling() {
    stopKitchenPolling();
    state.kitchenTimer = setInterval(loadKitchen, KIOSK_MODE ? 5000 : 15000);
}
function stopKitchenPolling() {
    if (state.kitchenTimer) clearInterval(state.kitchenTimer);
    state.kitchenTimer = null;
}

async function loadKitchen() {
    try {
        const rows = await call("get_kitchen_orders", {}, { silent: true });
        renderKitchen(rows);
    } catch (_) {}
}

function renderKitchen(rows) {
    const grid = $("#kitchen-grid");
    if (!rows.length) {
        grid.innerHTML = `<p class="empty-hint">Нема активни нарачки.</p>`;
        return;
    }
    const byOrder = new Map();
    for (const r of rows) {
        if (!byOrder.has(r.order_id)) {
            byOrder.set(r.order_id, { table_number: r.table_number, created_at: r.created_at, items: [] });
        }
        byOrder.get(r.order_id).items.push(r);
    }

    grid.innerHTML = Array.from(byOrder.entries()).map(([orderId, o]) => `
    <div class="ticket" data-order-id="${orderId}">
      <div class="ticket__head">
        <span class="ticket__table">Маса ${escapeHtml(o.table_number)}</span>
        <span class="ticket__time">${new Date(o.created_at).toLocaleTimeString("mk-MK", { hour: "2-digit", minute: "2-digit" })}</span>
      </div>
      <div class="ticket__items">
        ${o.items.map((it) => `
          <div class="ticket__item">
            <span>${escapeHtml(it.product_name)}</span>
            <span class="ticket__item-qty">×${it.quantity}</span>
          </div>
        `).join("")}
      </div>
      <div class="ticket__footer">
        <button class="btn btn--secondary btn--sm btn--block" data-action="ready">Спремно</button>
      </div>
    </div>
  `).join("");
}

$("#kitchen-grid").addEventListener("click", async (e) => {
    if (!e.target.closest('[data-action="ready"]')) return;
    const ticket = e.target.closest(".ticket");
    const orderId = Number(ticket.dataset.orderId);
    try {
        await call("mark_kitchen_order_ready", { orderId });
        ticket.remove();
        toast("Нарачката е означена како спремна.", "success");
    } catch (_) {}
});

$("#btn-refresh-kitchen").addEventListener("click", loadKitchen);

let inventoryCache = [];

async function loadInventory() {
    try {
        inventoryCache = await call("get_inventory", {}, { silent: true });
        renderInventory();
    } catch (_) {}
}

function renderInventory() {
    const onlyLow = $("#toggle-low-stock").checked;
    const query = $("#inventory-search").value.trim().toLowerCase();

    let rows = inventoryCache;
    if (onlyLow) rows = rows.filter((r) => r.quantity <= r.min_stock);
    if (query) rows = rows.filter((r) => r.name.toLowerCase().includes(query));

    const tbody = $("#inventory-table tbody");

    if (!rows.length) {
        tbody.innerHTML = `<tr><td colspan="5" class="empty-hint">Нема резултати.</td></tr>`;
        return;
    }

    tbody.innerHTML = rows.map((r) => {
        const low = r.quantity <= r.min_stock;
        return `
      <tr data-product-id="${r.product_id}">
        <td>${escapeHtml(r.name)}</td>
        <td class="num">${r.quantity}</td>
        <td class="num">${r.min_stock}</td>
        <td><span class="badge ${low ? "badge--warn" : "badge--ok"}">${low ? "Критично" : "Во ред"}</span></td>
        <td class="row-actions">
          ${state.isManager ? `<button class="btn btn--ghost btn--sm" data-action="restock">Дополни</button>` : ""}
        </td>
      </tr>`;
    }).join("");
}

$("#toggle-low-stock").addEventListener("change", renderInventory);
$("#inventory-search").addEventListener("input", renderInventory);

$("#inventory-table tbody").addEventListener("click", (e) => {
    if (!e.target.closest('[data-action="restock"]')) return;
    const tr = e.target.closest("tr");
    const productId = Number(tr.dataset.productId);
    const name = tr.children[0].textContent;
    openModal(`
    <h2>Дополни залиха</h2>
    <p class="modal-sub">${escapeHtml(name)}</p>
    <form class="modal-form" id="m-restock-form">
      <label class="field"><span>Количина</span><input type="number" id="m-restock-qty" min="1" value="1" required /></label>
      <div class="modal-actions">
        <button type="button" class="btn btn--secondary" data-modal-close>Откажи</button>
        <button type="submit" class="btn btn--primary">Дополни</button>
      </div>
    </form>
  `);
    $("#m-restock-form").addEventListener("submit", async (ev) => {
        ev.preventDefault();
        const qty = Number($("#m-restock-qty").value);
        try {
            await call("restock_product", { token: state.token, productId, quantity: qty });
            toast("Залихата е дополнета.", "success");
            closeModal();
            loadInventory();
        } catch (_) {}
    });
});

async function loadReports() {
    try {
        const [sales, orders, best, perf] = await Promise.all([
            call("today_sales", {}, { silent: true }),
            call("today_orders", {}, { silent: true }),
            call("best_product", {}, { silent: true }),
            call("employee_performance", { token: state.token }, { silent: true }),
        ]);
        $("#stat-today-sales").textContent = money(sales);
        $("#stat-today-orders").textContent = orders;
        $("#stat-best-product").textContent = best || "—";
        renderEmployeePerf(perf);
    } catch (_) {}
}

function renderEmployeePerf(rows) {
    const tbody = $("#employee-perf-table tbody");
    if (!rows.length) { tbody.innerHTML = `<tr><td colspan="3" class="empty-hint">Нема податоци.</td></tr>`; return; }
    tbody.innerHTML = rows.map((r) => `
    <tr>
      <td>${escapeHtml(r.name)}</td>
      <td class="num">${r.order_count}</td>
      <td class="num">${money(r.total)}</td>
    </tr>
  `).join("");
}

$("#btn-z-report").addEventListener("click", () => {
    openModal(`
    <h2>Затвори смена</h2>
    <p class="modal-sub">Ова ќе генерира Z-извештај за периодот од последното затворање. Дејството не може да се врати.</p>
    <div class="modal-actions">
      <button class="btn btn--secondary" data-modal-close>Откажи</button>
      <button class="btn btn--danger" id="m-confirm-z">Затвори смена</button>
    </div>
  `);
    $("#m-confirm-z").addEventListener("click", async () => {
        try {
            const r = await call("z_report", { token: state.token });
            openModal(`
        <h2>Z-извештај</h2>
        <div class="receipt">
          <div class="receipt__line"><span>Период од</span><span>${new Date(r.since).toLocaleString("mk-MK")}</span></div>
          <div class="receipt__line"><span>Број нарачки</span><span>${r.order_count}</span></div>
          <div class="receipt__rule"></div>
          <div class="receipt__total"><span>Вкупно</span><span>${money(r.total)}</span></div>
        </div>
        <div class="modal-actions">
          <button class="btn btn--secondary" id="m-print-z">Печати</button>
          <button class="btn btn--primary" data-modal-close>Готово</button>
        </div>
      `);
            $("#m-print-z").addEventListener("click", () => window.print());
            loadReports();
        } catch (_) {}
    });
});

$("#search-orders-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const query = $("#search-query").value.trim();
    const start = $("#search-start").value || null;
    const end = $("#search-end").value || null;
    try {
        const rows = await call("search_orders", { token: state.token, query, startDate: start, endDate: end });
        const tbody = $("#search-orders-table tbody");
        if (!rows.length) { tbody.innerHTML = `<tr><td colspan="4" class="empty-hint">Нема резултати.</td></tr>`; return; }
        tbody.innerHTML = rows.map((r) => `
      <tr>
        <td>${escapeHtml(r.invoice)}</td>
        <td>${new Date(r.date).toLocaleDateString("mk-MK")}</td>
        <td>${escapeHtml(r.employee)}</td>
        <td class="num">${money(r.amount)}</td>
      </tr>
    `).join("");
    } catch (_) {}
});

async function loadAdminMenu() {
    await loadCategories();
    renderCategoriesTable();
    await loadAllProducts();
    renderProductsTable();
}

function renderCategoriesTable() {
    const tbody = $("#categories-table tbody");
    if (!state.categories.length) { tbody.innerHTML = `<tr><td colspan="3" class="empty-hint">Нема категории.</td></tr>`; return; }
    tbody.innerHTML = state.categories.map((c) => `
    <tr data-id="${c.category_id}">
      <td>${escapeHtml(c.name)}</td>
      <td>${escapeHtml(c.description || "")}</td>
      <td class="row-actions">
        <button class="btn btn--ghost btn--sm" data-action="edit-cat">Уреди</button>
        <button class="btn btn--ghost btn--sm" data-action="delete-cat">Избриши</button>
      </td>
    </tr>
  `).join("");
}

function categoryModal(existing) {
    openModal(`
    <h2>${existing ? "Уреди категорија" : "Нова категорија"}</h2>
    <form class="modal-form" id="m-category-form">
      <label class="field"><span>Име</span><input type="text" id="m-cat-name" value="${existing ? escapeHtml(existing.name) : ""}" required /></label>
      <label class="field"><span>Опис</span><input type="text" id="m-cat-desc" value="${existing ? escapeHtml(existing.description || "") : ""}" /></label>
      <div class="modal-actions">
        <button type="button" class="btn btn--secondary" data-modal-close>Откажи</button>
        <button type="submit" class="btn btn--primary">Зачувај</button>
      </div>
    </form>
  `);
    $("#m-category-form").addEventListener("submit", async (e) => {
        e.preventDefault();
        const name = $("#m-cat-name").value.trim();
        const description = $("#m-cat-desc").value.trim();
        try {
            if (existing) {
                await call("update_category", { token: state.token, categoryId: existing.category_id, name, description });
            } else {
                await call("create_category", { token: state.token, name, description });
            }
            toast("Категоријата е зачувана.", "success");
            closeModal();
            await loadCategories();
            renderCategoriesTable();
        } catch (_) {}
    });
}

$("#btn-new-category").addEventListener("click", () => categoryModal(null));

$("#categories-table tbody").addEventListener("click", async (e) => {
    const tr = e.target.closest("tr");
    if (!tr) return;
    const id = Number(tr.dataset.id);
    const cat = state.categories.find((c) => c.category_id === id);
    if (e.target.closest('[data-action="edit-cat"]')) categoryModal(cat);
    if (e.target.closest('[data-action="delete-cat"]')) {
        openModal(`
      <h2>Избриши категорија</h2>
      <p class="modal-sub">Дали сте сигурни дека сакате да ја избришете „${escapeHtml(cat.name)}“?</p>
      <div class="modal-actions">
        <button class="btn btn--secondary" data-modal-close>Откажи</button>
        <button class="btn btn--danger" id="m-confirm-del-cat">Избриши</button>
      </div>
    `);
        $("#m-confirm-del-cat").addEventListener("click", async () => {
            try {
                await call("delete_category", { token: state.token, categoryId: id });
                toast("Категоријата е избришана.", "success");
                closeModal();
                await loadCategories();
                renderCategoriesTable();
            } catch (_) {}
        });
    }
});

function renderProductsTable() {
    const tbody = $("#products-table tbody");
    if (!state.products.length) { tbody.innerHTML = `<tr><td colspan="4" class="empty-hint">Нема производи.</td></tr>`; return; }

    const catName = (id) => state.categories.find((c) => c.category_id === id)?.name || "—";

    const query = $("#product-search").value.trim().toLowerCase();
    let rows = state.products;
    if (query) {
        rows = rows.filter((p) =>
            p.name.toLowerCase().includes(query) ||
            catName(p.category_id).toLowerCase().includes(query)
        );
    }

    if (!rows.length) { tbody.innerHTML = `<tr><td colspan="4" class="empty-hint">Нема резултати.</td></tr>`; return; }

    tbody.innerHTML = rows.map((p) => `
    <tr data-id="${p.product_id}">
      <td>${escapeHtml(p.name)}</td>
      <td>${escapeHtml(catName(p.category_id))}</td>
      <td class="num">${money(p.price)}</td>
      <td class="row-actions">
        <button class="btn btn--ghost btn--sm" data-action="edit-prod">Уреди</button>
        <button class="btn btn--ghost btn--sm" data-action="delete-prod">Избриши</button>
      </td>
    </tr>
  `).join("");
}

$("#product-search").addEventListener("input", renderProductsTable);

function productModal(existing) {
    const options = state.categories.map((c) => `<option value="${c.category_id}" ${existing && existing.category_id === c.category_id ? "selected" : ""}>${escapeHtml(c.name)}</option>`).join("");
    openModal(`
    <h2>${existing ? "Уреди производ" : "Нов производ"}</h2>
    <form class="modal-form" id="m-product-form">
      <label class="field"><span>Име</span><input type="text" id="m-prod-name" value="${existing ? escapeHtml(existing.name) : ""}" required /></label>
      <label class="field"><span>Категорија</span><select id="m-prod-cat" required>${options}</select></label>
      <label class="field"><span>Опис</span><input type="text" id="m-prod-desc" value="${existing ? escapeHtml(existing.description || "") : ""}" /></label>
      <label class="field"><span>Цена</span><input type="number" step="0.01" min="0" id="m-prod-price" value="${existing ? toNumber(existing.price) : ""}" required /></label>
      <div class="modal-actions">
        <button type="button" class="btn btn--secondary" data-modal-close>Откажи</button>
        <button type="submit" class="btn btn--primary">Зачувај</button>
      </div>
    </form>
  `);
    $("#m-product-form").addEventListener("submit", async (e) => {
        e.preventDefault();
        const name = $("#m-prod-name").value.trim();
        const categoryId = Number($("#m-prod-cat").value);
        const description = $("#m-prod-desc").value.trim();
        const price = Number($("#m-prod-price").value);
        try {
            if (existing) {
                await call("update_product", { token: state.token, productId: existing.product_id, categoryId, name, description, price });
            } else {
                await call("add_product", { token: state.token, categoryId, name, description, price });
            }
            toast("Производот е зачуван.", "success");
            closeModal();
            await loadAllProducts();
            renderProductsTable();
        } catch (_) {}
    });
}

$("#btn-new-product").addEventListener("click", () => {
    if (!state.categories.length) { toast("Прво креирајте категорија.", "error"); return; }
    productModal(null);
});

$("#products-table tbody").addEventListener("click", (e) => {
    const tr = e.target.closest("tr");
    if (!tr) return;
    const id = Number(tr.dataset.id);
    const prod = state.products.find((p) => p.product_id === id);
    if (e.target.closest('[data-action="edit-prod"]')) productModal(prod);
    if (e.target.closest('[data-action="delete-prod"]')) {
        openModal(`
      <h2>Избриши производ</h2>
      <p class="modal-sub">Дали сте сигурни дека сакате да го избришете „${escapeHtml(prod.name)}“?</p>
      <div class="modal-actions">
        <button class="btn btn--secondary" data-modal-close>Откажи</button>
        <button class="btn btn--danger" id="m-confirm-del-prod">Избриши</button>
      </div>
    `);
        $("#m-confirm-del-prod").addEventListener("click", async () => {
            try {
                await call("delete_product", { token: state.token, productId: id });
                toast("Производот е избришан.", "success");
                closeModal();
                await loadAllProducts();
                renderProductsTable();
            } catch (_) {}
        });
    }
});

let rolesCache = [];

async function loadEmployees() {
    try {
        const [emps, roles] = await Promise.all([
            call("get_employees", { token: state.token }, { silent: true }),
            call("get_roles", {}, { silent: true }),
        ]);
        rolesCache = roles;
        renderEmployeesTable(emps);
    } catch (_) {}
}

function renderEmployeesTable(emps) {
    const tbody = $("#employees-table tbody");
    if (!emps.length) { tbody.innerHTML = `<tr><td colspan="4" class="empty-hint">Нема вработени.</td></tr>`; return; }
    tbody.innerHTML = emps.map((e) => `
    <tr data-id="${e.id}">
      <td>${escapeHtml(e.first_name)} ${escapeHtml(e.last_name)}</td>
      <td>
        <select data-action="role-select">
          ${rolesCache.map((r) => `<option value="${r.role_id}" ${r.role_id === e.role_id ? "selected" : ""}>${escapeHtml(r.role_name)}</option>`).join("")}
        </select>
      </td>
      <td><span class="badge ${e.active ? "badge--ok" : "badge--muted"}">${e.active ? "Активен" : "Неактивен"}</span></td>
      <td class="row-actions">
        <button class="btn btn--ghost btn--sm" data-action="reset-pin">ПИН</button>
        <button class="btn btn--ghost btn--sm" data-action="toggle-active">${e.active ? "Деактивирај" : "Активирај"}</button>
        <button class="btn btn--ghost btn--sm" data-action="delete-emp">Избриши</button>
      </td>
    </tr>
  `).join("");
}

$("#employees-table tbody").addEventListener("change", async (e) => {
    if (!e.target.closest('[data-action="role-select"]')) return;
    const tr = e.target.closest("tr");
    const userId = Number(tr.dataset.id);
    const roleId = Number(e.target.value);
    try {
        await call("update_employee_role", { token: state.token, userId, roleId });
        toast("Улогата е ажурирана.", "success");
    } catch (_) {}
});

$("#employees-table tbody").addEventListener("click", async (e) => {
    const tr = e.target.closest("tr");
    if (!tr) return;
    const userId = Number(tr.dataset.id);

    if (e.target.closest('[data-action="toggle-active"]')) {
        try {
            await call("toggle_employee", { token: state.token, userId });
            toast("Статусот е променет.", "success");
            loadEmployees();
        } catch (_) {}
    }

    if (e.target.closest('[data-action="reset-pin"]')) {
        openModal(`
      <h2>Ресетирај ПИН</h2>
      <form class="modal-form" id="m-reset-pin-form">
        <label class="field"><span>Нов ПИН</span><input type="password" id="m-new-pin" inputmode="numeric" required /></label>
        <div class="modal-actions">
          <button type="button" class="btn btn--secondary" data-modal-close>Откажи</button>
          <button type="submit" class="btn btn--primary">Зачувај</button>
        </div>
      </form>
    `);
        $("#m-reset-pin-form").addEventListener("submit", async (ev) => {
            ev.preventDefault();
            const newPin = $("#m-new-pin").value;
            try {
                await call("reset_employee_pin", { token: state.token, userId, newPin });
                toast("ПИН-от е ресетиран.", "success");
                closeModal();
            } catch (_) {}
        });
    }

    if (e.target.closest('[data-action="delete-emp"]')) {
        const nameCell = tr.querySelector("td");
        const empName = nameCell ? nameCell.textContent.trim() : "вработениот";
        openModal(`
      <h2>Избриши вработен</h2>
      <p class="modal-sub">Дали сте сигурни дека сакате да го избришете „${escapeHtml(empName)}“? Оваа акција е неповратна.</p>
      <div class="modal-actions">
        <button class="btn btn--secondary" data-modal-close>Откажи</button>
        <button class="btn btn--danger" id="m-confirm-del-emp">Избриши</button>
      </div>
    `);
        $("#m-confirm-del-emp").addEventListener("click", async () => {
            try {
                await call("delete_employee", { token: state.token, userId });
                toast("Вработениот е избришан.", "success");
                closeModal();
                loadEmployees();
            } catch (_) {}
        });
    }
});

$("#btn-new-employee").addEventListener("click", () => {
    const options = rolesCache.map((r) => `<option value="${r.role_id}">${escapeHtml(r.role_name)}</option>`).join("");
    openModal(`
    <h2>Нов вработен</h2>
    <form class="modal-form" id="m-new-emp-form">
      <div class="field-row">
        <label class="field"><span>Име</span><input type="text" id="m-emp-first" required /></label>
        <label class="field"><span>Презиме</span><input type="text" id="m-emp-last" required /></label>
      </div>
      <label class="field"><span>Улога</span><select id="m-emp-role" required>${options}</select></label>
      <label class="field"><span>ПИН</span><input type="password" id="m-emp-pin" inputmode="numeric" required /></label>
      <div class="modal-actions">
        <button type="button" class="btn btn--secondary" data-modal-close>Откажи</button>
        <button type="submit" class="btn btn--primary">Додади</button>
      </div>
    </form>
  `);
    $("#m-new-emp-form").addEventListener("submit", async (e) => {
        e.preventDefault();
        const firstName = $("#m-emp-first").value.trim();
        const lastName = $("#m-emp-last").value.trim();
        const roleId = Number($("#m-emp-role").value);
        const pin = $("#m-emp-pin").value;
        try {
            await call("add_employee", { token: state.token, firstName, lastName, pin, roleId });
            toast("Вработениот е додаден.", "success");
            closeModal();
            loadEmployees();
        } catch (_) {}
    });
});

async function loadSettings() {
    try {
        const s = await call("get_settings", {}, { silent: true });
        state.settings = s;
        $("#set-restaurant-name").value = s.restaurant_name || "";
        $("#set-vat-rate").value = toNumber(s.vat_rate);
        $("#set-currency").value = s.currency || "";
        $("#set-printer-config").value = s.printer_config || "";
    } catch (_) {}
}

$("#settings-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    try {
        await call("update_settings", {
            token: state.token,
            restaurantName: $("#set-restaurant-name").value.trim(),
            vatRate: Number($("#set-vat-rate").value),
            currency: $("#set-currency").value.trim(),
            printerConfig: $("#set-printer-config").value.trim(),
        });
        toast("Поставките се зачувани.", "success");
        loadSettingsQuiet();
    } catch (_) {}
});

tickClock();
loadSettingsQuiet();
$("#view-login").classList.add("is-active");