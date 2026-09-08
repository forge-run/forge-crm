/*
 * forecast — the manager view: weighted open pipeline (amount x stage
 * probability), the per-stage rollup, the win rate, and the lead-to-won
 * funnel.
 *
 * The op that needed the float read TWICE over, on both sides of one
 * multiplication. `opportunities.amount` is a `number` column, and
 * `pipeline_stages.probability` is an `integer` one read as a float and
 * divided by 100 — which is exactly HS-9's widening arm: the column is
 * stored as a JSON INTEGER (`10`, `25`, `100`), the read answers `10.0`
 * rather than the default, and `prob` is `0.1` rather than `0.0`. A read
 * that answered the default here would report a weighted pipeline of zero
 * on every stage, in silence, and it would look like an empty quarter
 * rather than like a bug.
 */
package forge.crm;

import forge.Op;
import forge.OpContext;
import forge.Storage;
import forge.Value;

import java.util.List;

public final class Forecast {

    public record StageRoll(
        String stage,
        long count,
        double raw_value,
        double weighted_value
    ) {}

    public record Funnel(long leads, long qualified, long open_opportunities, long won) {}

    public record ForecastOutput(
        double weighted_total,
        List<StageRoll> by_stage,
        long won,
        long lost,
        double win_rate,
        Funnel funnel
    ) {}

    /** One stage's rollup. A `static` helper rather than a block-bodied
     * lambda, which the subset refuses (`FL2033`). */
    private static StageRoll roll(Value st, List<Value> open) {
        String sid = st.get("id", "");
        double prob = st.get("probability", 0.0) / 100.0;
        double raw = 0.0;
        long n = 0L;
        for (Value o : open) {
            if (o.get("stage_id", "").equals(sid)) {
                raw = raw + o.get("amount", 0.0);
                n = n + 1L;
            }
        }
        // The Rust carried the raw `name` Value here and fell back to
        // `json!("")`; `pipeline_stages.name` is a required Text column, so
        // the string read is the same answer on every row that exists. The
        // one shape they part on is a stored null, which the schema does not
        // permit: the Rust would write `null` and this writes `""`.
        return new StageRoll(st.get("name", ""), n, raw, raw * prob);
    }

    @Op("forecast")
    public static ForecastOutput forecast(OpContext ctx, Value input) {
        List<Value> stages = Storage.rows(Storage.query(Value.obj(
            "from", "pipeline_stages",
            "sort", Value.list(Value.obj("column", "position", "direction", "asc")),
            "limit", 50L
        )));
        List<Value> open = Storage.rows(Storage.query(Value.obj(
            "from", "opportunities",
            "filter", Value.obj("column", "status", "op", "eq", "value", "open"),
            "select", Value.list(Value.of("id"), Value.of("amount"), Value.of("stage_id")),
            "limit", 2000L
        )));
        List<StageRoll> by_stage = stages.stream().map(st -> roll(st, open)).toList();
        double weighted_total = 0.0;
        for (StageRoll r : by_stage) {
            weighted_total = weighted_total + r.weighted_value();
        }

        List<Value> won_rows = Storage.rows(Storage.query(Value.obj(
            "from", "opportunities",
            "filter", Value.obj("column", "status", "op", "eq", "value", "won"),
            "select", Value.list(Value.of("id")),
            "limit", 5000L
        )));
        List<Value> lost_rows = Storage.rows(Storage.query(Value.obj(
            "from", "opportunities",
            "filter", Value.obj("column", "status", "op", "eq", "value", "lost"),
            "select", Value.list(Value.of("id")),
            "limit", 5000L
        )));
        long won = won_rows.size();
        long lost = lost_rows.size();
        double win_rate = 0.0;
        if (won + lost > 0L) {
            // FLOAT division. Java's `/` on two longs truncates and the
            // dialect's floors (`FL2027`); the Rust cast both sides to f64,
            // so both operands widen here first — which is the int-to-float
            // widening the checker admits at a declared `double` slot.
            double won_f = won;
            double total_f = won + lost;
            win_rate = won_f / total_f;
        }

        List<Value> leads_all = Storage.rows(Storage.query(Value.obj(
            "from", "leads",
            "select", Value.list(Value.of("id")),
            "limit", 10000L
        )));
        List<Value> leads_qualified = Storage.rows(Storage.query(Value.obj(
            "from", "leads",
            "filter", Value.obj("column", "status", "op", "eq", "value", "qualified"),
            "select", Value.list(Value.of("id")),
            "limit", 10000L
        )));

        return new ForecastOutput(
            weighted_total,
            by_stage,
            won,
            lost,
            win_rate,
            new Funnel(leads_all.size(), leads_qualified.size(), open.size(), won)
        );
    }
}
