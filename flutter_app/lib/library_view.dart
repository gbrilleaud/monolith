import 'dart:io';

import 'package:flutter/material.dart';

import 'src/rust/flutter_api.dart';
import 'src/rust/library_service.dart';

const _background = Color(0xFF080C15);
const _surface = Color(0xFF141B29);
const _accent = Color(0xFF67D6DA);

/// Présentation pure : les jeux, filtres et autorisations viennent de Rust.
class MonolithLibraryView extends StatefulWidget {
  const MonolithLibraryView({
    super.key,
    required this.sections,
    required this.onLaunch,
    this.onDownload,
    this.onEmulators,
    this.update,
    this.onUpdate,
    this.onSearchChanged,
    this.onRefresh,
    this.identity,
    this.onLogout,
    this.onLogin,
    this.busy = false,
    this.notice,
  });

  final List<LibrarySection> sections;
  final ValueChanged<String> onLaunch;
  final ValueChanged<int>? onDownload;
  final VoidCallback? onEmulators;
  final FlutterUpdateStatus? update;
  final VoidCallback? onUpdate;
  final ValueChanged<String>? onSearchChanged;
  final VoidCallback? onRefresh;
  final FlutterIdentity? identity;
  final VoidCallback? onLogout;
  final VoidCallback? onLogin;
  final bool busy;
  final String? notice;

  @override
  State<MonolithLibraryView> createState() => _MonolithLibraryViewState();
}

class _MonolithLibraryViewState extends State<MonolithLibraryView> {
  bool _list = false;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: _background,
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final wide = constraints.maxWidth >= 900;
            final content = CustomScrollView(
              slivers: [
                SliverToBoxAdapter(child: _header(context, wide)),
                if (widget.busy)
                  const SliverToBoxAdapter(
                    child: LinearProgressIndicator(color: _accent),
                  ),
                if (widget.notice != null)
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 24,
                        vertical: 8,
                      ),
                      child: Text(
                        widget.notice!,
                        style: const TextStyle(color: Color(0xFFFFC578)),
                      ),
                    ),
                  ),
                if (widget.sections.isEmpty)
                  SliverFillRemaining(
                    hasScrollBody: false,
                    child: Center(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          const Icon(
                            Icons.videogame_asset_outlined,
                            color: _accent,
                            size: 54,
                          ),
                          const SizedBox(height: 16),
                          const Text(
                            'Votre bibliothèque attend sa première synchronisation.',
                          ),
                          const SizedBox(height: 16),
                          OutlinedButton(
                            onPressed: widget.onRefresh,
                            child: const Text('Actualiser'),
                          ),
                        ],
                      ),
                    ),
                  ),
                for (final section in widget.sections)
                  SliverToBoxAdapter(
                    child: AnimatedSwitcher(
                      duration: const Duration(milliseconds: 280),
                      switchInCurve: Curves.easeOutCubic,
                      child: KeyedSubtree(
                        key: ValueKey(
                          '${section.systemId}:${section.games.map((game) => game.gameId).join(',')}',
                        ),
                        child: _section(context, section),
                      ),
                    ),
                  ),
                const SliverToBoxAdapter(child: SizedBox(height: 36)),
              ],
            );
            return wide
                ? Row(
                    children: [
                      SizedBox(
                        width: 224,
                        child: Container(
                          color: _surface,
                          child: SafeArea(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                const Padding(
                                  padding: EdgeInsets.all(26),
                                  child: Text(
                                    'MONOLITH',
                                    style: TextStyle(
                                      fontSize: 21,
                                      fontWeight: FontWeight.w900,
                                      letterSpacing: 3,
                                      color: _accent,
                                    ),
                                  ),
                                ),
                                const Divider(color: Colors.white12),
                                const Padding(
                                  padding: EdgeInsets.fromLTRB(24, 18, 0, 10),
                                  child: Text(
                                    'CONSOLES',
                                    style: TextStyle(
                                      fontSize: 11,
                                      letterSpacing: 2,
                                      color: Colors.white54,
                                    ),
                                  ),
                                ),
                                for (final section in widget.sections)
                                  ListTile(
                                    leading: const Icon(
                                      Icons.sports_esports_outlined,
                                    ),
                                    title: Text(
                                      section.name,
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                    subtitle: Text(
                                      '${section.games.length} jeux',
                                    ),
                                  ),
                              ],
                            ),
                          ),
                        ),
                      ),
                      Expanded(child: content),
                    ],
                  )
                : content;
          },
        ),
      ),
    );
  }

  Widget _header(BuildContext context, bool wide) => Padding(
    padding: EdgeInsets.fromLTRB(wide ? 40 : 20, 28, wide ? 40 : 20, 24),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (!wide)
          const Text(
            'MONOLITH',
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w900,
              letterSpacing: 3,
              color: _accent,
            ),
          ),
        const SizedBox(height: 26),
        if (widget.identity == null && widget.onLogin != null)
          Row(
            children: [
              const Text('Mode hors ligne'),
              const Spacer(),
              TextButton(
                onPressed: widget.onLogin,
                child: const Text('Connexion'),
              ),
            ],
          ),
        if (widget.identity != null)
          Row(
            children: [
              Text(widget.identity!.username),
              const Spacer(),
              TextButton(
                onPressed: widget.onLogout,
                child: const Text('Déconnexion'),
              ),
            ],
          ),
        Text(
          'Votre bibliothèque',
          style: Theme.of(context).textTheme.headlineMedium
              ?.copyWith(fontWeight: FontWeight.w800, letterSpacing: -0.8),
        ),
        const SizedBox(height: 7),
        const Text(
          'Toutes vos aventures, au même endroit.',
          style: TextStyle(color: Colors.white60),
        ),
        const SizedBox(height: 22),
        TextField(
          onChanged: widget.onSearchChanged,
          decoration: InputDecoration(
            hintText: 'Rechercher un jeu (* et ? acceptés)',
            prefixIcon: const Icon(Icons.search, color: _accent),
            suffixIcon: IconButton(
              tooltip: 'Actualiser',
              onPressed: widget.onRefresh,
              icon: const Icon(Icons.refresh),
            ),
            filled: true,
            fillColor: _surface,
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(14),
              borderSide: BorderSide.none,
            ),
          ),
        ),
        const SizedBox(height: 12),
        if (widget.update != null && widget.update!.phase != 'unsupported') ...[
          SizedBox(
            width: double.infinity,
            child: OutlinedButton.icon(
              onPressed:
                  widget.update!.phase == 'checking' ||
                      widget.update!.phase == 'downloading'
                  ? null
                  : widget.onUpdate,
              icon: Icon(
                widget.update!.phase == 'ready'
                    ? Icons.system_update_alt
                    : Icons.update,
              ),
              label: Text(switch (widget.update!.phase) {
                'available' =>
                  'Télécharger Monolith ${widget.update!.version ?? ''}',
                'ready' => 'Installer la dernière version de Monolith',
                'checking' => 'Recherche de mise à jour…',
                'downloading' => 'Téléchargement de Monolith…',
                'error' => 'Réessayer la mise à jour de Monolith',
                _ => 'Vérifier les mises à jour de Monolith',
              }),
            ),
          ),
          if (widget.update!.message != null)
            Text(
              widget.update!.message!,
              style: const TextStyle(color: Color(0xFFFFC578)),
            ),
          const SizedBox(height: 12),
        ],
        Row(
          children: [
            if (widget.onEmulators != null)
              OutlinedButton.icon(
                onPressed: widget.onEmulators,
                icon: const Icon(Icons.settings_input_component),
                label: const Text('Émulateurs'),
              ),
            const Spacer(),
            IconButton(
              tooltip: 'Vue grille',
              onPressed: () => setState(() => _list = false),
              icon: Icon(
                Icons.grid_view,
                color: _list ? Colors.white54 : _accent,
              ),
            ),
            IconButton(
              tooltip: 'Vue liste',
              onPressed: () => setState(() => _list = true),
              icon: Icon(
                Icons.view_list,
                color: _list ? _accent : Colors.white54,
              ),
            ),
          ],
        ),
      ],
    ),
  );

  Widget _section(BuildContext context, LibrarySection section) => Padding(
    padding: const EdgeInsets.only(bottom: 30),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 0, 20, 16),
          child: Row(
            children: [
              Icon(
                _list ? Icons.view_list : Icons.grid_view_rounded,
                color: _accent,
                size: 22,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  section.name,
                  style: Theme.of(context).textTheme.titleLarge
                      ?.copyWith(fontWeight: FontWeight.bold),
                ),
              ),
              Text(
                '${section.games.length} JEUX',
                style: const TextStyle(
                  color: Colors.white54,
                  letterSpacing: 1,
                  fontSize: 11,
                ),
              ),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20),
          child: _list
              ? Column(
                  children: [
                    for (final game in section.games)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: ListTile(
                          tileColor: _surface,
                          title: Text(game.title),
                          subtitle: Text(
                            game.description,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          trailing: _gameAction(
                            game,
                            widget.onLaunch,
                            widget.onDownload,
                            widget.busy,
                          ),
                        ),
                      ),
                  ],
                )
              : LayoutBuilder(
                  builder: (context, constraints) {
                    final columns = ((constraints.maxWidth + 14) / 210)
                        .floor()
                        .clamp(1, 12);
                    return GridView.builder(
                      shrinkWrap: true,
                      physics: const NeverScrollableScrollPhysics(),
                      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                        crossAxisCount: columns,
                        mainAxisSpacing: 14,
                        crossAxisSpacing: 14,
                        mainAxisExtent: 410,
                      ),
                      itemCount: section.games.length,
                      itemBuilder: (context, index) => _GameCard(
                        game: section.games[index],
                        onLaunch: widget.onLaunch,
                        onDownload: widget.onDownload,
                        busy: widget.busy,
                      ),
                    );
                  },
                ),
        ),
      ],
    ),
  );
}

Widget _gameAction(
  LibraryGame game,
  ValueChanged<String> onLaunch,
  ValueChanged<int>? onDownload,
  bool busy,
) => FilledButton.icon(
  onPressed: busy
      ? null
      : game.available
      ? () => onLaunch(game.path)
      : game.gameId != null && onDownload != null
      ? () => onDownload(game.gameId!)
      : null,
  icon: Icon(
    game.available ? Icons.play_arrow_rounded : Icons.download_rounded,
    size: 18,
  ),
  label: Text(game.available ? 'Lancer' : 'Télécharger'),
);

class _GameCard extends StatelessWidget {
  const _GameCard({
    required this.game,
    required this.onLaunch,
    this.onDownload,
    required this.busy,
  });
  final LibraryGame game;
  final ValueChanged<String> onLaunch;
  final ValueChanged<int>? onDownload;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final cover = game.coverUri;
    Widget image = const Center(
      child: Icon(
        Icons.videogame_asset_outlined,
        color: Colors.white38,
        size: 58,
      ),
    );
    if (cover != null && cover.isNotEmpty) {
      if (cover.startsWith('https://') || cover.startsWith('http://')) {
        image = Image.network(
          cover,
          fit: BoxFit.contain,
          errorBuilder: (_, _, _) => const Center(
            child: Icon(
              Icons.image_not_supported_outlined,
              color: Colors.white38,
            ),
          ),
        );
      } else if (cover.startsWith('file://')) {
        image = Image.file(
          File.fromUri(Uri.parse(cover)),
          fit: BoxFit.contain,
          errorBuilder: (_, _, _) => const Center(
            child: Icon(
              Icons.image_not_supported_outlined,
              color: Colors.white38,
            ),
          ),
        );
      }
    }
    return Container(
      decoration: BoxDecoration(
        color: _surface,
        border: Border.all(color: Colors.white10),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SizedBox(
            height: 290,
            child: ColoredBox(color: const Color(0xFF202A3C), child: image),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(13, 11, 13, 0),
            child: SizedBox(
              height: 40,
              child: Text(
                game.title,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ),
          const Spacer(),
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
            child: SizedBox(
              height: 38,
              child: _gameAction(game, onLaunch, onDownload, busy),
            ),
          ),
        ],
      ),
    );
  }
}
