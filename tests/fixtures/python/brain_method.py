"""Function that is both complex AND large — Brain Method."""


def process_data_pipeline(records, config, output_format):
    results = []
    errors = []
    skipped = 0
    processed = 0
    total = len(records)

    for record in records:
        if record is None:
            skipped += 1
            continue
        if not record.get("id"):
            errors.append({"error": "missing_id", "record": record})
            continue
        if record.get("deleted"):
            skipped += 1
            continue

        record_type = record.get("type", "unknown")
        if record_type == "user":
            if record.get("email"):
                record["email"] = record["email"].lower().strip()
            if record.get("name"):
                record["name"] = record["name"].strip()
            if record.get("age") and record["age"] < 0:
                errors.append({"error": "invalid_age", "id": record["id"]})
                continue
        elif record_type == "order":
            if record.get("total") and record["total"] < 0:
                errors.append({"error": "negative_total", "id": record["id"]})
                continue
            if record.get("items"):
                for item in record["items"]:
                    if item.get("quantity", 0) <= 0:
                        item["quantity"] = 1
            if record.get("discount") and record["discount"] > record.get("total", 0):
                record["discount"] = record["total"]
        elif record_type == "product":
            if not record.get("sku"):
                errors.append({"error": "missing_sku", "id": record["id"]})
                continue
            if record.get("price") and record["price"] < 0:
                record["price"] = 0

        if config.get("validate_dates"):
            if record.get("created_at"):
                try:
                    parse_date(record["created_at"])
                except ValueError:
                    errors.append({"error": "bad_date", "id": record["id"]})
                    continue
            if record.get("updated_at"):
                try:
                    parse_date(record["updated_at"])
                except ValueError:
                    record["updated_at"] = None

        results.append(record)
        processed += 1

    return {
        "results": results,
        "errors": errors,
        "stats": {
            "total": total,
            "processed": processed,
            "skipped": skipped,
            "error_count": len(errors),
        },
    }


def parse_date(s):
    pass
