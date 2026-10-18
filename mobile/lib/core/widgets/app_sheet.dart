import 'package:flutter/material.dart';

/// Opens [builder] as a modal bottom sheet, the way Rental Command opens its
/// sheets (`features/onboarding/go_live_sheet.dart` `showGoLiveSheet`).
Future<T?> showAppSheet<T>(BuildContext context, WidgetBuilder builder) =>
    showModalBottomSheet<T>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      showDragHandle: true,
      builder: builder,
    );

/// A sheet that opens at [initialSize] of the screen and drags up to full
/// height: an [AppSheetHeader] that stays put, then [itemCount] rows from
/// [itemBuilder]. Padding from Rental Command's
/// `core/widgets/tabbed_form_sheet.dart` `TabbedFormSheet`.
class AppSheet extends StatelessWidget {
  const AppSheet({
    super.key,
    required this.title,
    this.subtitle,
    this.onBack,
    this.initialSize = 0.5,
    required this.itemCount,
    required this.itemBuilder,
  });

  final String title;
  final String? subtitle;

  /// Shows a back arrow in place of the close button.
  final VoidCallback? onBack;
  final double initialSize;
  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: initialSize,
      minChildSize: 0.25,
      maxChildSize: 1,
      builder: (context, controller) => CustomScrollView(
        controller: controller,
        slivers: [
          // Pinned inside the scroll view so dragging it also resizes the sheet.
          PinnedHeaderSliver(
            child: ColoredBox(
              color: theme.bottomSheetTheme.modalBackgroundColor!,
              // 16 more each side, as the model sheet's list pads its header.
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: AppSheetHeader(
                  title: title,
                  subtitle: subtitle,
                  onBack: onBack,
                ),
              ),
            ),
          ),
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(20, 4, 20, 32),
            sliver: SliverList.builder(
              itemCount: itemCount,
              itemBuilder: itemBuilder,
            ),
          ),
        ],
      ),
    );
  }
}

/// A sheet's header: close (or back) button, then the title and subtitle in
/// the middle. Rental Command's `TabbedFormSheet` title row.
class AppSheetHeader extends StatelessWidget {
  const AppSheetHeader({
    super.key,
    required this.title,
    this.subtitle,
    this.onBack,
  });

  final String title;
  final String? subtitle;

  /// Shows a back arrow in place of the close button.
  final VoidCallback? onBack;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
      child: Row(
        children: [
          IconButton(
            tooltip: onBack == null ? 'Close' : 'Back',
            icon: Icon(onBack == null ? Icons.close : Icons.arrow_back),
            onPressed: onBack ?? () => Navigator.of(context).pop(),
          ),
          Expanded(
            child: Column(
              children: [
                Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.titleMedium?.copyWith(
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                if (subtitle case final subtitle?)
                  Text(
                    subtitle,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
              ],
            ),
          ),
          // Balances the button so the title sits in the middle.
          const SizedBox(width: 48),
        ],
      ),
    );
  }
}
