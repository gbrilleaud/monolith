import 'dart:async';

import 'package:flutter/material.dart';

import 'library_view.dart';
import 'src/rust/flutter_api.dart' as api;
import 'src/rust/frb_generated.dart';
import 'src/rust/library_service.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const MonolithApp());
}

class MonolithApp extends StatelessWidget {
  const MonolithApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Monolith',
    debugShowCheckedModeBanner: false,
    theme: ThemeData.dark(useMaterial3: true).copyWith(
      scaffoldBackgroundColor: const Color(0xFF080C15),
      colorScheme: ColorScheme.fromSeed(
        seedColor: const Color(0xFF67D6DA),
        brightness: Brightness.dark,
        surface: const Color(0xFF141B29),
      ),
      textTheme: ThemeData.dark().textTheme.apply(fontFamily: 'Roboto'),
    ),
    home: const _MonolithHome(),
  );
}

class _MonolithHome extends StatefulWidget {
  const _MonolithHome();
  @override
  State<_MonolithHome> createState() => _MonolithHomeState();
}

class _MonolithHomeState extends State<_MonolithHome> {
  List<LibrarySection> _sections = const [];
  String _query = '';
  String? _notice;
  String? _dataDirectory;
  bool _busy = true;
  Timer? _debounce;
  int _request = 0;

  @override
  void initState() {
    super.initState();
    _initialize();
  }

  Future<void> _initialize() async {
    try {
      _dataDirectory = await api.defaultDataDirectory();
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
      if (mounted && request == _request) {
        setState(() {
          _sections = sections;
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

  @override
  void dispose() {
    _debounce?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => MonolithLibraryView(
    sections: _sections,
    onLaunch: _launch,
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
