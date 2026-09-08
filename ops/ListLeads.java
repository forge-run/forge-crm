/*
 * list_leads — unconverted leads, newest first, optionally by status.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class ListLeads {

    public record ListLeadsOutput(long total, List<Value> rows) {}

    @Op("list_leads")
    public static ListLeadsOutput listLeads(OpContext ctx, Value input) {
        Value spec = Value.obj(
            "from", "leads",
            "filter", Value.obj("column", "converted", "op", "eq", "value", false),
            "sort", Value.list(Value.obj("column", "created_at", "direction", "desc")),
            "limit", 200L
        );
        String status = input.get("status", "");
        if (!status.equals("")) {
            spec.set("filter", Value.obj("and", Value.list(
                Value.obj("column", "converted", "op", "eq", "value", false),
                Value.obj("column", "status", "op", "eq", "value", status)
            )));
        }
        List<Value> rows = Storage.rows(Storage.query(spec));
        return new ListLeadsOutput(rows.size(), rows);
    }
}
