/*
 * close_opportunity — close a deal won or lost, a one-click operational
 * action. Anything that is not "lost" closes the deal won, which is the
 * Rust op's own two-arm normalisation.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

public final class CloseOpportunity {

    public record CloseOpportunityOutput(long committed) {}

    @Op("close_opportunity")
    public static CloseOpportunityOutput closeOpportunity(OpContext ctx, Value input) {
        String id = input.get("id", "");
        if (id.equals("")) {
            throw OpError.bad_request("id required");
        }
        String given = input.get("outcome", "");
        if (given.equals("")) {
            given = "won";
        }
        String outcome = "won";
        if (given.equals("lost")) {
            outcome = "lost";
        }
        Value res = Storage.mutate(Value.obj(
            "table", "opportunities",
            "op", "update",
            "filter", Value.obj("column_equals", Value.obj("column", "id", "value", id)),
            "patch", Value.obj("status", outcome)
        ));
        return new CloseOpportunityOutput(res.get("committed", 0L));
    }
}
