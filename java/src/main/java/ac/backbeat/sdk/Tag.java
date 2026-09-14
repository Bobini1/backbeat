package ac.backbeat.sdk;

import java.util.Objects;

/**
 * Tags are used across backbeat collections for collection creators to strap on extra
 * arbitrary key->value metadata.
 *
 * You can use tags to influence how you process a collection, like deciding what requirements
 * are on a course, or letting people mark levels in a table as "color=red".
 */
public final class Tag {
    private final String key;
    private final String value;

    public Tag(String key, String value) { this.key = key; this.value = value; }
    public String key() { return key; }
    public String value() { return value; }

    @Override public boolean equals(Object other) {
        return other instanceof Tag tag && Objects.equals(key, tag.key) && Objects.equals(value, tag.value);
    }
    @Override public int hashCode() { return Objects.hash(key, value); }
    @Override public String toString() { return "Tag[key=" + key + ", value=" + value + "]"; }
}
