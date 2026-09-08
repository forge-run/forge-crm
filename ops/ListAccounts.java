/*
 * list_accounts — the accounts list page's data binding.
 *
 * Reads need no tenant filter: `accounts`' row_security scopes every query
 * to the caller's tenant. `search` is an optional ilike over `name`.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class ListAccounts {

    public record ListAccountsOutput(long total, List<Value> rows) {}

    @Op("list_accounts")
    public static ListAccountsOutput listAccounts(OpContext ctx, Value input) {
        Value spec = Value.obj(
            "from", "accounts",
            "sort", Value.list(Value.obj("column", "created_at", "direction", "desc")),
            "limit", 200L
        );
        String term = input.get("search", "");
        if (!term.equals("")) {
            spec.set("filter", Value.obj("column", "name", "op", "ilike", "value", "%" + term + "%"));
        }
        List<Value> rows = Storage.rows(Storage.query(spec));
        return new ListAccountsOutput(rows.size(), rows);
    }
}
