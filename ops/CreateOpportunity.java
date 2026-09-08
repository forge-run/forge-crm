/*
 * create_opportunity — insert one deal, tenant + owner stamped from the
 * trusted session.
 *
 * The op HS-9 exists for, in its smallest form: `amount` is a `number`
 * column and the Rust it replaces read it with `as_f64()`, which answers for
 * a JSON integer as well as a JSON float. `input.get("amount", …)` is that
 * read now (`GetKind::Float`), and a caller who posts `120000` rather than
 * `120000.0` still stores `120000.0` — the widening, on the write side.
 */
package forge.crm;

import forge.Forge;
import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class CreateOpportunity {

    public record CreateOpportunityOutput(Value id, long committed) {}

    /** `insert` — the shared write path, spelled per source because the
     * dialect compiles every file in isolation (FL2001: no shared helper
     * class). tenant_id arrives via `$auth:tenant_id`; owner_id is the
     * authenticated caller. */
    private static CreateOpportunityOutput insert(String table, Value row, String uid, Value input) {
        String tenant = input.get("tenant_id", "");
        if (tenant.equals("")) {
            throw OpError.forbidden();
        }
        if (uid.equals("")) {
            throw OpError.forbidden();
        }
        row.set("tenant_id", tenant);
        row.set("owner_id", uid);
        Value res = Storage.mutate(Value.obj("table", table, "op", "insert", "rows", Value.list(row)));
        List<Value> ids = res.get("ids", List.of());
        Value id = Value.of("");
        if (!ids.isEmpty()) {
            id = Forge.nth(ids, 0L);
        }
        return new CreateOpportunityOutput(id, res.get("committed", 0L));
    }

    @Op("create_opportunity")
    public static CreateOpportunityOutput createOpportunity(OpContext ctx, Value input) {
        String name = input.get("name", "");
        if (name.equals("")) {
            throw OpError.bad_request("name required");
        }
        Value row = Value.obj("name", name);
        String account_id = input.get("account_id", "");
        if (!account_id.equals("")) {
            row.set("account_id", account_id);
        }
        String stage_id = input.get("stage_id", "");
        if (!stage_id.equals("")) {
            row.set("stage_id", stage_id);
        }
        String close_date = input.get("close_date", "");
        if (!close_date.equals("")) {
            row.set("close_date", close_date);
        }
        String currency = input.get("currency", "");
        if (!currency.equals("")) {
            row.set("currency", currency);
        }
        // The Rust set `amount` only when `as_f64()` ANSWERED — so an absent
        // key and a present-but-not-a-number key both left the column unset,
        // and neither wrote a zero. `has()` cannot tell those two apart from
        // a stored number, so the test is two reads with different defaults:
        // the float read answers the STORED value when there is one, and the
        // default when there is not, so the two agree exactly when a number
        // was there to read. Nothing about the sentinels is special — any two
        // distinct doubles say the same thing.
        double probe_low = input.get("amount", -1.0);
        double probe_high = input.get("amount", -2.0);
        if (probe_low == probe_high) {
            row.set("amount", probe_low);
        }
        return insert("opportunities", row, ctx.user_id(), input);
    }
}
