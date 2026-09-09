/*
 * get_contact — one contact, its account, and its activity timeline.
 */
package forge.crm;

import forge.Forge;
import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class GetContact {

    public record GetContactOutput(Value contact, Value account, List<Value> activities) {}

    @Op("get_contact")
    public static GetContactOutput getContact(OpContext ctx, Value input) {
        String id = input.get("id", "");
        if (id.equals("")) {
            throw OpError.bad_request("id required");
        }
        List<Value> contactRows = Storage.rows(Storage.query(Value.obj(
            "from", "contacts",
            "filter", Value.obj("column", "id", "op", "eq", "value", id),
            "limit", 1L
        )));
        if (contactRows.isEmpty()) {
            throw OpError.not_found();
        }
        Value contact = Forge.nth(contactRows, 0L);

        Value account = Value.none();
        String accountId = contact.get("account_id", "");
        if (!accountId.equals("")) {
            List<Value> accountRows = Storage.rows(Storage.query(Value.obj(
                "from", "accounts",
                "filter", Value.obj("column", "id", "op", "eq", "value", accountId),
                "limit", 1L
            )));
            if (!accountRows.isEmpty()) {
                account = Forge.nth(accountRows, 0L);
            }
        }

        List<Value> activities = Storage.rows(Storage.query(Value.obj(
            "from", "activities",
            "filter", Value.obj("and", Value.list(
                Value.obj("column", "related_type", "op", "eq", "value", "contact"),
                Value.obj("column", "related_id", "op", "eq", "value", id)
            )),
            "sort", Value.list(Value.obj("column", "created_at", "direction", "desc")),
            "limit", 50L
        )));
        return new GetContactOutput(contact, account, activities);
    }
}
