import { button, clear, el, formatDate, icon } from "../dom.js";
import { RowStatus, SortKey } from "../store.js";

const ROW_HEIGHT = 44;
const MAX_STAGGER_INDEX = 14;
const EMPTY_VIEWPORT_HEIGHT = 320;

export function createResults(controller) {
  const heading = el("h2", { class: "text-base font-semibold text-slate-700 dark:text-slate-100", "data-i18n": "list.heading" });
  const countText = el("p", { class: "text-xs text-slate-500 dark:text-slate-400" });

  const headerRow = el("div", {
    class: "hidden grid-cols-[auto_minmax(0,1fr)_9.5rem_6.5rem] items-center gap-3 border-b border-slate-200/70 px-3 pb-2 pt-1 md:grid"
  });
  const sortableHeader = (key, labelKey) =>
    button({
      label: controller.t(labelKey),
      className: "sort-header justify-center px-2",
      title: controller.t(labelKey),
      onClick: () => controller.onSort(key)
    });

  const pathHeader = sortableHeader(SortKey.Path, "list.path");
  const modifiedHeader = sortableHeader(SortKey.Modified, "list.modified");
  const statusHeader = el("span", { class: "px-2 text-left text-slate-500 dark:text-slate-400", text: controller.t("list.status") });
  headerRow.append(el("span", { class: "w-4" }), pathHeader, modifiedHeader, statusHeader);

  const viewport = el("div", {
    class: "list-viewport",
    style: `min-height:${EMPTY_VIEWPORT_HEIGHT}px`,
    dataset: { role: "result-viewport" }
  });
  const spacer = el("div", { class: "list-spacer" });
  const rowsLayer = el("div", { class: "list-window" });
  const emptyState = el("div", {
    class: "flex min-h-[16rem] flex-col items-center justify-center gap-2 px-6 py-10 text-center"
  });
  spacer.append(rowsLayer);
  viewport.append(spacer, emptyState);

  const node = el("section", { class: "surface-enter flex min-h-0 flex-1 flex-col gap-3 p-4 sm:p-5", style: "--surface-index:2" }, [
    el("div", { class: "flex flex-wrap items-center justify-between gap-2" }, [heading, countText]),
    headerRow,
    viewport
  ]);

  let rows = [];
  let selected = new Set();
  let sortable = { key: SortKey.Path, ascending: true };
  let forceStructureRender = true;
  let lastVisibleCount = -1;
  let lastTotalCount = -1;
  const animatedPaths = new Set();

  function renderRows() {
    spacer.style.height = `${rows.length * ROW_HEIGHT}px`;
    clear(rowsLayer);
    for (let index = 0; index < rows.length; index++) {
      rowsLayer.append(createRow(rows[index], index));
    }
  }

  function createRow(row, index) {
    const isPending = row.status === RowStatus.Pending;
    const isSelected = selected.has(row.path);
    const shouldAnimate = !animatedPaths.has(row.path);
    const checkbox = el("input", {
      type: "checkbox",
      class: `row-checkbox${isSelected ? " is-checked" : ""}`,
      checked: isSelected,
      disabled: !isPending,
      "aria-label": row.path,
      dataset: { role: "row-select" },
      onchange: () => controller.onToggleRow(row.path)
    });

    const rowNode = el(
      "div",
      {
        class: `result-row${shouldAnimate ? " row-enter" : ""} grid h-11 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 border-b border-slate-100/80 px-3 last:border-b-0 dark:border-slate-800/60 md:grid-cols-[auto_minmax(0,1fr)_9.5rem_6.5rem]`,
        style: `--item-index:${Math.min(index, MAX_STAGGER_INDEX)}`,
        dataset: { selected: String(isSelected), role: "result-row", path: row.path }
      },
      [
        checkbox,
        el("span", {
          class: "truncate text-sm text-slate-700 dark:text-slate-200",
          title: row.path,
          text: row.path
        }),
        el("span", { class: "hidden text-xs tabular-nums text-slate-500 md:block", text: formatDate(row.modified) }),
        el("span", { class: "flex justify-end md:justify-start" }, [
          el("span", { class: `status-pill status-${row.status}`, text: controller.t(statusKey(row.status)) })
        ])
      ]
    );

    rowNode.addEventListener("contextmenu", (event) => {
      event.preventDefault();
      controller.onRowMenu(row.path, { x: event.clientX, y: event.clientY });
    });
    animatedPaths.add(row.path);
    return rowNode;
  }

  function updateSortIndicators() {
    for (const [header, key] of [
      [pathHeader, SortKey.Path],
      [modifiedHeader, SortKey.Modified]
    ]) {
      const active = sortable.key === key;
      header.dataset.active = String(active);
      header.setAttribute("aria-sort", active ? (sortable.ascending ? "ascending" : "descending") : "none");
    }
  }

  function renderEmptyState() {
    clear(emptyState);
    emptyState.append(
      el("span", { class: "text-slate-300 dark:text-slate-600" }, [icon("folder", 40)]),
      el("p", {
        class: "text-sm text-slate-500 dark:text-slate-400",
        text: controller.t(controller.currentFilter() ? "list.filtered" : "list.empty")
      })
    );
  }

  return {
    node,
    setData(nextRows, nextSelected, nextSortable, counts) {
      const structureChanged =
        forceStructureRender ||
        rows !== nextRows ||
        !sameSet(selected, nextSelected) ||
        !sameSortable(sortable, nextSortable);
      if (structureChanged) {
        rows = nextRows;
        selected = nextSelected;
        sortable = nextSortable;
        forceStructureRender = false;
      }
      emptyState.classList.toggle("hidden", rows.length > 0);
      if (rows.length === 0) {
        clear(rowsLayer);
        spacer.style.height = "0px";
        renderEmptyState();
        if (counts.total === 0) {
          animatedPaths.clear();
        }
      }
      if (counts.visible !== lastVisibleCount || counts.total !== lastTotalCount) {
        lastVisibleCount = counts.visible;
        lastTotalCount = counts.total;
        countText.textContent = buildCount(counts);
      }
      headerRow.classList.toggle("hidden", rows.length === 0);
      updateSortIndicators();
      if (structureChanged) {
        renderRows();
      }
    },
    viewState() {
      return {
        rows: rows.length,
        selected: selected.size,
        sortKey: sortable.key,
        sortAscending: sortable.sortAscending
      };
    },
    updateLabels() {
      heading.textContent = controller.t("list.heading");
      pathHeader.querySelector("span").textContent = controller.t("list.path");
      modifiedHeader.querySelector("span").textContent = controller.t("list.modified");
      statusHeader.textContent = controller.t("list.status");
      forceStructureRender = true;
      lastVisibleCount = -1;
      lastTotalCount = -1;
      renderEmptyState();
      renderRows();
    }
  };

  function buildCount(counts) {
    if (counts.total === 0) {
      return "";
    }
    return counts.visible === counts.total
      ? controller.t("list.count", { count: counts.total })
      : controller.t("list.filteredCount", { visible: counts.visible, total: counts.total });
  }
}

function sameSet(left, right) {
  if (!left || !right || left.size !== right.size) {
    return false;
  }
  for (const value of left) {
    if (!right.has(value)) {
      return false;
    }
  }
  return true;
}

function sameSortable(left, right) {
  return left?.key === right?.key && left?.ascending === right?.ascending;
}

function statusKey(status) {
  switch (status) {
    case RowStatus.Deleted:
      return "list.deleted";
    case RowStatus.Remaining:
      return "list.remaining";
    case RowStatus.Failed:
      return "list.failed";
    default:
      return "list.pending";
  }
}
