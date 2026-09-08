/*
 * list_contacts — the contacts list page's data binding.
 *
 * Optional `account_id` and `search` filters; the account is followed so the
 * list can render the company name without a second read.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class ListContacts {

    public record ListContactsOutput(long total, List<Value> rows) {}

    @Op("list_contacts")
    public static ListContactsOutput listContacts(OpContext ctx, Value input) {
        Value spec = Value.obj(
            "from", "contacts",
            "sort", Value.list(Value.obj("column", "created_at", "direction", "desc")),
            "limit", 200L,
            "follow", Value.list(Value.obj(
                "relationship", "account_id",
                "select", Value.list(Value.of("id"), Value.of("name"))
            ))
        );
        String account = input.get("account_id", "");
        String term = input.get("search", "");
        Value accountFilter = Value.obj("column", "account_id", "op", "eq", "value", account);
        Value termFilter = Value.obj("column", "last_name", "op", "ilike", "value", "%" + term + "%");
        if (!account.equals("") && !term.equals("")) {
            spec.set("filter", Value.obj("and", Value.list(accountFilter, termFilter)));
        } else if (!account.equals("")) {
            spec.set("filter", accountFilter);
        } else if (!term.equals("")) {
            spec.set("filter", termFilter);
        }
        List<Value> rows = Storage.rows(Storage.query(spec));
        return new ListContactsOutput(rows.size(), rows);
    }
}
