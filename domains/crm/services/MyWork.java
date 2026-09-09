/*
 * my_work — the caller's open activities, soonest due first.
 *
 * No server clock dependency; the page buckets by `due_at` client-side.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class MyWork {

    public record MyWorkOutput(List<Value> rows, long total) {}

    @Op("my_work")
    public static MyWorkOutput myWork(OpContext ctx, Value input) {
        // The Rust op puts `ctx.user_id` (an Option) straight into the filter,
        // so an anonymous caller filters on JSON null. `Value.none()` is that
        // same request byte for byte.
        Value owner = Value.none();
        String uid = ctx.user_id();
        if (!uid.equals("")) {
            owner = Value.of(uid);
        }
        List<Value> rows = Storage.rows(Storage.query(Value.obj(
            "from", "activities",
            "filter", Value.obj("and", Value.list(
                Value.obj("column", "completed", "op", "eq", "value", false),
                Value.obj("column", "owner_id", "op", "eq", "value", owner)
            )),
            "sort", Value.list(Value.obj("column", "due_at", "direction", "asc")),
            "limit", 200L
        )));
        return new MyWorkOutput(rows, rows.size());
    }
}
