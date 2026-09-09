/*
 * create_contact — insert one contact, tenant + owner stamped from the
 * trusted session.
 */
package forge.crm;

import forge.Forge;
import forge.Op;
import forge.OpContext;
import forge.OpError;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class CreateContact {

    public record CreateContactOutput(Value id, long committed) {}

    private static CreateContactOutput insert(String table, Value row, String uid, Value input) {
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
        return new CreateContactOutput(id, res.get("committed", 0L));
    }

    @Op("create_contact")
    public static CreateContactOutput createContact(OpContext ctx, Value input) {
        String first = input.get("first_name", "");
        if (first.equals("")) {
            throw OpError.bad_request("first_name required");
        }
        Value row = Value.obj("first_name", first);
        String lastName = input.get("last_name", "");
        if (!lastName.equals("")) {
            row.set("last_name", lastName);
        }
        String email = input.get("email", "");
        if (!email.equals("")) {
            row.set("email", email);
        }
        String phone = input.get("phone", "");
        if (!phone.equals("")) {
            row.set("phone", phone);
        }
        String title = input.get("title", "");
        if (!title.equals("")) {
            row.set("title", title);
        }
        String accountId = input.get("account_id", "");
        if (!accountId.equals("")) {
            row.set("account_id", accountId);
        }
        return insert("contacts", row, ctx.user_id(), input);
    }
}
