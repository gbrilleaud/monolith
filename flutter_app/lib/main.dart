import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';

import 'emulator_settings.dart';
import 'controller_settings.dart';
import 'library_view.dart';
import 'login_view.dart';
import 'style_settings.dart';
import 'style_theme.dart';
import 'src/rust/flutter_api.dart' as api;
import 'src/rust/flutter_api.dart' show FlutterIdentity, FlutterUpdateStatus;
import 'src/rust/frb_generated.dart';
import 'src/rust/library_service.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const MonolithApp());
}

class MonolithApp extends StatefulWidget {
  const MonolithApp({super.key});

  @override
  State<MonolithApp> createState() => _MonolithAppState();
}

class _MonolithAppState extends State<MonolithApp> {
  StyleTheme _style = StyleTheme.fallback;
  String? _dataDirectory;
  int _styleGeneration = 0;

  @override
  void initState() {
    super.initState();
    _loadStyle();
  }

  Future<void> _loadStyle() async {
    final generation = _styleGeneration;
    try {
      final root = await api.defaultDataDirectory();
      if (mounted) setState(() => _dataDirectory = root);
      final snapshot = await api.styleSnapshotJson(dataDirectory: root);
      if (mounted && generation == _styleGeneration) {
        setState(() {
          _style = StyleTheme.fromSnapshot(snapshot);
        });
      }
    } catch (_) {
      // Keep the bundled dark presentation if the local JSON is unreadable.
    }
  }

  void _styleChanged(String snapshot) {
    if (mounted) {
      setState(() {
        _styleGeneration++;
        _style = StyleTheme.fromSnapshot(snapshot);
      });
    }
  }

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Monolith',
    debugShowCheckedModeBanner: false,
    theme: _style.toThemeData(dataDirectory: _dataDirectory),
    home: _MonolithHome(style: _style, onStyleChanged: _styleChanged),
  );
}

class _MonolithHome extends StatefulWidget {
  const _MonolithHome({required this.style, required this.onStyleChanged});
  final StyleTheme style;
  final ValueChanged<String> onStyleChanged;
  @override
  State<_MonolithHome> createState() => _MonolithHomeState();
}

class _MonolithHomeState extends State<_MonolithHome> {
  List<LibrarySection> _sections = const [];
  FlutterIdentity? _identity;
  String _query = '';
  String? _notice;
  String? _dataDirectory;
  String _backendUrl = '';
  bool _offline = false;
  bool _busy = true;
  Timer? _debounce;
  Timer? _updateTimer;
  FlutterUpdateStatus? _update;
  bool _updatePolling = false;
  int _request = 0;

  @override
  void initState() {
    super.initState();
    _initialize();
  }

  Future<void> _initialize() async {
    try {
      _dataDirectory = await api.defaultDataDirectory();
      _updateTimer = Timer.periodic(
        const Duration(seconds: 2),
        (_) => _pollUpdate(),
      );
      unawaited(_pollUpdate());
      _backendUrl = await api.backendUrl(dataDirectory: _dataDirectory!);
      await _refresh();
    } catch (error) {
      if (mounted) {
        setState(() {
          _busy = false;
          _notice = 'Bibliothèque : $error';
        });
      }
    }
  }

  Future<void> _pollUpdate() async {
    final directory = _dataDirectory;
    if (directory == null || _updatePolling || !mounted) return;
    _updatePolling = true;
    try {
      final status = await api.flutterUpdateStatus(dataDirectory: directory);
      if (mounted && status != _update) setState(() => _update = status);
    } catch (error) {
      if (mounted) setState(() => _notice = 'Mise à jour Monolith : $error');
    } finally {
      _updatePolling = false;
    }
  }

  Future<void> _updateMonolith() async {
    final directory = _dataDirectory;
    final phase = _update?.phase;
    if (directory == null) return;
    try {
      if (phase == 'ready') {
        final confirmed = await showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: const Text('Installer Monolith ?'),
            content: const Text(
              'Le client va se fermer. La mise à jour vérifiée sera installée, '
              'puis Monolith redémarrera automatiquement.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context, false),
                child: const Text('Annuler'),
              ),
              FilledButton(
                onPressed: () => Navigator.pop(context, true),
                child: const Text('Installer'),
              ),
            ],
          ),
        );
        if (confirmed != true || !mounted) return;
        await api.installFlutterUpdate(dataDirectory: directory);
        // L'auxiliaire attend la libération des DLL avant de remplacer le bundle.
        exit(0);
      } else if (phase == 'available') {
        await api.downloadFlutterUpdate(dataDirectory: directory);
      } else if (phase != 'checking' && phase != 'downloading') {
        await api.checkFlutterUpdate(dataDirectory: directory);
      }
      await _pollUpdate();
    } catch (error) {
      if (mounted) {
        setState(() => _notice = 'Mise à jour Monolith refusée : $error');
      }
    }
  }

  Future<void> _refresh() async {
    final directory = _dataDirectory;
    if (directory == null) return;
    final request = ++_request;
    setState(() {
      _busy = true;
      _notice = null;
    });
    try {
      final sections = await api.loadLibrary(
        dataDirectory: directory,
        query: _query,
      );
      final identity = await api.sessionIdentity(dataDirectory: directory);
      if (mounted && request == _request) {
        setState(() {
          _sections = sections;
          _identity = identity;
          _busy = false;
        });
      }
    } catch (error) {
      if (mounted && request == _request) {
        setState(() {
          _busy = false;
          _notice = 'Chargement impossible : $error';
        });
      }
    }
  }

  Future<void> _launch(String path) async {
    try {
      await api.launchLibraryRom(dataDirectory: _dataDirectory!, path: path);
    } catch (error) {
      if (mounted) setState(() => _notice = 'Lancement refusé : $error');
    }
  }

  Future<void> _download(int gameId) async {
    final directory = _dataDirectory;
    if (directory == null || _busy) return;
    setState(() {
      _busy = true;
      _notice = 'Téléchargement et vérification de la ROM…';
    });
    try {
      final path = await api.downloadLibraryRom(
        dataDirectory: directory,
        gameId: gameId,
      );
      if (!mounted) return;
      await _refresh();
      if (mounted) {
        setState(() => _notice = 'ROM téléchargée et vérifiée : $path');
      }
    } catch (error) {
      if (mounted) setState(() => _notice = 'Téléchargement refusé : $error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _openEmulators() async {
    final directory = _dataDirectory;
    if (directory == null) return;
    try {
      final config = await api.emulatorConfigText(dataDirectory: directory);
      if (!mounted) return;
      final saved = await showDialog<bool>(
        context: context,
        builder: (_) => EmulatorSettings(
          initialConfig: config,
          onSave: (value) =>
              api.saveEmulatorConfig(dataDirectory: directory, value: value),
          onInstall: (name) =>
              api.installEmulator(dataDirectory: directory, name: name),
        ),
      );
      if (mounted && saved == true) {
        setState(() => _notice = 'Profils émulateurs enregistrés');
      }
    } catch (error) {
      if (mounted) {
        setState(() => _notice = 'Configuration émulateurs : $error');
      }
    }
  }

  Future<void> _openStyle() async {
    final root = _dataDirectory;
    if (root == null) return;
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => Dialog(
        child: SizedBox(
          width: 680,
          height: MediaQuery.sizeOf(dialogContext).height * 0.82,
          child: StyleSettings(
            snapshotJson: jsonEncode(widget.style.snapshot),
            onChanged: widget.onStyleChanged,
            onSelect: (id) => api.selectStyle(dataDirectory: root, id: id),
            onAnimation: (enabled) => api.setCartridgeAnimation(
              dataDirectory: root,
              enabled: enabled,
            ),
            onRefresh: () => api.refreshStyles(dataDirectory: root),
            onReset: () => api.resetStyle(dataDirectory: root),
            onSaveCustom: (json) =>
                api.saveCustomStyle(dataDirectory: root, json: json),
          ),
        ),
      ),
    );
  }

  Future<void> _openControllers() async {
    final directory = _dataDirectory;
    if (directory == null) return;
    try {
      final config = await api.controllerConfigText(dataDirectory: directory);
      final devices = await api.detectedControllers();
      if (!mounted) return;
      final saved = await showDialog<bool>(
        context: context,
        builder: (_) => ControllerSettings(
          initialConfig: config,
          detected: devices,
          onSave: (value) =>
              api.saveControllerConfig(dataDirectory: directory, value: value),
          onReset: (id, device) => api.resetControllerConfig(
            dataDirectory: directory,
            systemId: id,
            deviceId: device,
          ),
          onReload: () => api.controllerConfigText(dataDirectory: directory),
        ),
      );
      if (mounted && saved == true) {
        setState(() => _notice = 'Profils des manettes enregistrés');
      }
    } catch (error) {
      if (mounted) setState(() => _notice = 'Configuration manettes : $error');
    }
  }

  Future<void> _saveBackendUrl(String value) async {
    final directory = _dataDirectory;
    if (directory == null) return;
    setState(() {
      _busy = true;
      _notice = null;
    });
    try {
      await api.saveBackendUrl(dataDirectory: directory, value: value);
      final url = await api.backendUrl(dataDirectory: directory);
      if (!mounted) return;
      setState(() {
        _backendUrl = url;
        _offline = false;
      });
      await _refresh();
    } catch (error) {
      if (mounted) {
        setState(() {
          _busy = false;
          _notice = 'Serveur invalide : $error';
        });
      }
    }
  }

  Future<void> _login(String username, String password) async {
    final directory = _dataDirectory;
    if (directory == null) return;
    setState(() {
      _busy = true;
      _notice = null;
    });
    try {
      await api.loginLocal(
        dataDirectory: directory,
        username: username,
        password: password,
      );
      if (!mounted) return;
      setState(() => _offline = false);
      await _refresh();
    } catch (error) {
      if (mounted) {
        setState(() {
          _busy = false;
          _notice = 'Connexion refusée : $error';
        });
      }
    }
  }

  Future<void> _logout() async {
    final directory = _dataDirectory;
    if (directory == null) return;
    try {
      await api.logout(dataDirectory: directory);
      if (!mounted) return;
      setState(() => _offline = false);
      await _refresh();
    } catch (error) {
      if (mounted) setState(() => _notice = 'Déconnexion impossible : $error');
    }
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _updateTimer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_identity == null && !_offline) {
      return MonolithLoginView(
        backendUrl: _backendUrl,
        onSaveBackendUrl: _saveBackendUrl,
        onLogin: _login,
        onOffline: () => setState(() => _offline = true),
        onStyle: _dataDirectory == null ? null : _openStyle,
        busy: _busy,
        notice: _notice,
      );
    }
    return MonolithLibraryView(
      sections: _sections,
      identity: _identity,
      onLogout: _identity == null ? null : _logout,
      onLogin: _identity == null
          ? () => setState(() => _offline = false)
          : null,
      onLaunch: _launch,
      onDownload: _identity?.role == 'standard' || _identity?.role == 'admin'
          ? _download
          : null,
      onEmulators: _openEmulators,
      onControllers: _openControllers,
      onStyle: _openStyle,
      cartridgeAnimation: widget.style.cartridgeAnimation,
      update: _update,
      onUpdate: _updateMonolith,
      busy: _busy,
      notice: _notice,
      onRefresh: _refresh,
      onSearchChanged: (query) {
        _query = query;
        _debounce?.cancel();
        _debounce = Timer(const Duration(milliseconds: 250), _refresh);
      },
    );
  }
}
