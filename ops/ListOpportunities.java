/*
 * list_opportunities — the deal list, soonest close first, account followed.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class ListOpportunities {

    public record ListOpportunitiesOutput(long total, List<Value> rows) {}

    @Op("list_opportunities")
    public static ListOpportunitiesOutput listOpportunities(OpContext ctx, Value input) {
        Value spec = Value.obj(
            "from", "opportunities",
            "sort", Value.list(Value.obj("column", "close_date", "direction", "asc")),
            "follow", Value.list(Value.obj(
                "relationship", "account_id",
                "select", Value.list(Value.of("id"), Value.of("name"))
            )),
            "limit", 300L
        );
        String status = input.get("status", "");
        if (!status.equals("")) {
            spec.set("filter", Value.obj("column", "status", "op", "eq", "value", status));
        }
        List<Value> rows = Storage.rows(Storage.query(spec));
        return new ListOpportunitiesOutput(rows.size(), rows);
    }
}
