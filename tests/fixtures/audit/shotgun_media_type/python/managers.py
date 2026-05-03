def get_queryset(media_type, base):
    if media_type == "tv":
        return base.prefetch_related("seasons")
    if media_type == "season":
        return base.prefetch_related("episodes")
    if media_type == "movie":
        return base.select_related("studio")
    return base


def annotate(media_type, queryset):
    if media_type == "movie":
        return queryset.annotate(progress=1)
    return queryset
