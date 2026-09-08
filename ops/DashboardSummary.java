/*
 * dashboard_summary — the landing page's five counters plus the recent
 * activity feed.
 *
 * `open_pipeline_value` is why this op waited for HS-9: it sums
 * `opportunities.amount`, a `number` column, and until `GetKind::Float`
 * there was no way to read one out of a JSON value on any band.
 *
 * TWO deliberate differences from the Rust this replaces, both stated rather
 * than hidden:
 *
 *  * `seed_diag` arrives on the INPUT. The demo-workspace bootstrap is
 *    imperative — per-insert error swallowing, a diagnostic string built by
 *    mutation, and `ctx.tenant_id`, which the dialect's context surface does
 *    not carry — so it stays in the host crate (`src/seed.rs`) and runs from
 *    `lib.rs` BEFORE this op, exactly where the Rust ran it. The op echoes
 *    what the host reports. The value and its position on the wire are
 *    unchanged.
 *  * the Rust's `qrows` swallowed a failed read to an empty list so one bad
 *    aggregate never blanked the dashboard. The dialect has no `try` and is
 *    not getting one (`FL2034`), so a read that FAILS now faults the op
 *    rather than answering zero. Under any workspace with storage granted
 *    the two are identical, which is what the CRM swap gate compares; the
 *    difference is on the storage-denied path alone, and a dashboard that
 *    silently reports zero open deals because a read was refused is the
 *    worse of the two answers.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class DashboardSummary {

    public record DashboardSummaryOutput(
        long open_opportunities,
        double open_pipeline_value,
        long won_this_period,
        long contacts_total,
        long leads_open,
        List<Value> recent_activities,
        String seed_diag
    ) {}

    @Op("dashboard_summary")
    public static DashboardSummaryOutput dashboardSummary(OpContext ctx, Value input) {
        List<Value> open = Storage.rows(Storage.query(Value.obj(
            "from", "opportunities",
            "filter", Value.obj("column", "status", "op", "eq", "value", "open"),
            "select", Value.list(Value.of("id"), Value.of("amount")),
            "limit", 1000L
        )));
        // `-0.0`, not `0.0`, and it is not a typo. Rust's
        // `Iterator::sum::<f64>()` folds from NEGATIVE zero — the identity
        // that is exact for every operand including `-0.0` itself — so the
        // Rust this replaces answered `-0.0` for a stage with no deals, and
        // serde writes that to the wire as `-0.0`. An accumulator seeded at
        // `0.0` agrees on every non-empty sum and parts on exactly the empty
        // one, which is what the CRM swap gate caught on `Closed Won`.
        double pipeline_value = -0.0;
        for (Value o : open) {
            pipeline_value = pipeline_value + o.get("amount", 0.0);
        }
        List<Value> won = Storage.rows(Storage.query(Value.obj(
            "from", "opportunities",
            "filter", Value.obj("column", "status", "op", "eq", "value", "won"),
            "select", Value.list(Value.of("id")),
            "limit", 1000L
        )));
        List<Value> contacts = Storage.rows(Storage.query(Value.obj(
            "from", "contacts",
            "select", Value.list(Value.of("id")),
            "limit", 5000L
        )));
        List<Value> leads_open = Storage.rows(Storage.query(Value.obj(
            "from", "leads",
            "filter", Value.obj("column", "converted", "op", "eq", "value", false),
            "select", Value.list(Value.of("id")),
            "limit", 5000L
        )));
        List<Value> recent = Storage.rows(Storage.query(Value.obj(
            "from", "activities",
            "sort", Value.list(Value.obj("column", "created_at", "direction", "desc")),
            "limit", 8L
        )));
        return new DashboardSummaryOutput(
            open.size(),
            pipeline_value,
            won.size(),
            contacts.size(),
            leads_open.size(),
            recent,
            input.get("seed_diag", "")
        );
    }
}
