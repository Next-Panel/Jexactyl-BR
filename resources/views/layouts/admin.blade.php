<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <title>{{ config('app.name', 'Jexactyl') }} - @yield('title')</title>
        <meta content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no" name="viewport">
        <meta name="_token" content="{{ csrf_token() }}">

        <link rel="apple-touch-icon" sizes="180x180" href="/favicons/apple-touch-icon.png">
        <link rel="icon" type="image/png" href="/favicons/favicon-32x32.png" sizes="32x32">
        <link rel="icon" type="image/png" href="/favicons/favicon-16x16.png" sizes="16x16">
        <link rel="manifest" href="/favicons/manifest.json">
        <link rel="mask-icon" href="/favicons/safari-pinned-tab.svg" color="#bc6e3c">
        <link rel="shortcut icon" href="/favicons/favicon.ico">
        <meta name="msapplication-config" content="/favicons/browserconfig.xml">
        <meta name="theme-color" content="#0e4688">

        <script src="https://unpkg.com/feather-icons"></script>

        @include('layouts.scripts')

        @section('scripts')
            {!! Theme::css('vendor/select2/select2.min.css?t={cache-version}') !!}
            {!! Theme::css('vendor/bootstrap/bootstrap.min.css?t={cache-version}') !!}
            {!! Theme::css('vendor/adminlte/admin.min.css?t={cache-version}') !!}
            <!-- Habilita o sistema Sidebar da Jexactyl-Brasil -->
            <link rel="stylesheet" href="/themes/default/vendor/{{ config('sidebar.admin', 'sidebar') }}/sidebar.css?t={cache-version}') !!}
            {!! Theme::css('vendor/adminlte/colors/skin-blue.min.css?t={cache-version}') !!}
            {!! Theme::css('vendor/sweetalert/sweetalert.min.css?t={cache-version}') !!}
            {!! Theme::css('vendor/animate/animate.min.css?t={cache-version}') !!}
            <!-- Habilita o tema admin do jexactyl -->
            <link rel="stylesheet" href="/themes/{{ config('theme.admin', 'jexactyl') }}/css/{{ config('theme.admin', 'jexactyl') }}.css">

            <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/4.7.0/css/font-awesome.min.css">
            <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/ionicons/2.0.1/css/ionicons.min.css">
            @show
        </head>
        <body class="hold-transition skin-blue fixed sidebar-mini">
            <div class="wrapper">
                <header class="main-header">
                <nav class="navbar navbar-static-top">
                    <a href="#" class="sidebar-toggle" data-toggle="push-menu" role="button">
                        <span class="sr-only">Toggle navigation</span>
                        <span class="icon-bar"></span>
                        <span class="icon-bar"></span>
                        <span class="icon-bar"></span>
                    </a>
                </header>
                <aside class="main-sidebar">
                    <!-- Novo codigo-->
                    <a style="margin-left: 20px; margin-top: 10px;">
                    <img src="https://avatars.githubusercontent.com/u/91636558" width="48" height="48" />
                                        <span class="transition-text" style="margin-left: 8px; font-weight: bold; font-size: 20px; color: #FFF;">{{ config('app.name', 'Jexactyl') }}</span>
                                    </a>
                            <!------------>
                    <section class="sidebar">
                        <ul class="sidebar-menu">
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.index') ?: 'desactive' }}">
                                <a href="{{ route('index')}}">
                                    <i data-feather="home" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Voltar ao Painel</span> 
                                </a>
                            </li>
                        <!-- Novo codigo-->
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.index') ?: 'active' }}">
                                <a href="{{ route('admin.index')}}">
                                    <i data-feather="tool" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Configurações</span> 
                                </a>
                            </li>
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.tickets') ?: 'active' }}">
                                <a href="{{ route('admin.tickets.index')}}">
                                    <i class="fa fa-ticket" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Tickets</span>
                                </a>
                            </li>
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.api') ?: 'active' }}">
                                <a href="{{ route('admin.api.index')}}">
                                    <i data-feather="git-branch" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Aplicação de API</span>
                                </a>
                            </li>
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.databases') ?: 'active' }}">
                                <a href="{{ route('admin.databases') }}">
                                    <i data-feather="database" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Banco de Dados</span>
                                </a>
                            </li>
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.locations') ?: 'active' }}">
                                <a href="{{ route('admin.locations') }}">
                                    <i data-feather="navigation" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Localizações</span>
                                </a>
                            </li>
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.nodes') ?: 'active' }}">
                                <a href="{{ route('admin.nodes') }}">
                                    <i data-feather="layers" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Nodes</span>
                                </a>
                            </li>
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.servers') ?: 'active' }}">
                                <a href="{{ route('admin.servers') }}">
                                    <i data-feather="server" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Servidores</span>
                                </a>
                            </li>
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.users') ?: 'active' }}">
                                <a href="{{ route('admin.users') }}">
                                    <i data-feather="users" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Usuários</span>
                                </a>
                            </li>
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.mounts') ?: 'active' }}">
                                <a href="{{ route('admin.mounts') }}">
                                    <i data-feather="hard-drive" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Montagens</span>
                                </a>
                            </li>
                            <li class="{{ ! starts_with(Route::currentRouteName(), 'admin.nests') ?: 'active' }}">
                                <a href="{{ route('admin.nests') }}">
                                    <i data-feather="archive" style="margin-left: 12px;"></i>
                                    <span style="margin-left: 4px; font-weight: bold; font-size: 18px;">Nests</span>
                                </a>
                            </li>
                        </ul>
                    </section>
                </aside>
                <div class="content-wrapper">
                    <section class="content-header">
                        @yield('content-header')
                    </section>
                    <section class="content">
                        <div class="row">
                            <div class="col-xs-12">
                                @if (count($errors) > 0)
                                    <div class="alert alert-danger">
                                        Ocorreu um erro ao validar os dados fornecidos.<br><br>
                                        <ul>
                                            @foreach ($errors->all() as $error)
                                                <li>{{ $error }}</li>
                                            @endforeach
                                        </ul>
                                    </div>
                                @endif
                                @foreach (Alert::getMessages() as $type => $messages)
                                    @foreach ($messages as $message)
                                        <div class="alert alert-{{ $type }} alert-dismissable" role="alert">
                                            {!! $message !!}
                                        </div>
                                    @endforeach
                                @endforeach
                            </div>
                        </div>
                        @yield('content')
                    </section>
                </div>
            </div>
            @section('footer-scripts')
                <script src="/js/keyboard.polyfill.js" type="application/javascript"></script>
                <script>keyboardeventKeyPolyfill.polyfill();</script>

                {!! Theme::js('vendor/jquery/jquery.min.js?t={cache-version}') !!}
                {!! Theme::js('vendor/sweetalert/sweetalert.min.js?t={cache-version}') !!}
                {!! Theme::js('vendor/bootstrap/bootstrap.min.js?t={cache-version}') !!}
                {!! Theme::js('vendor/slimscroll/jquery.slimscroll.min.js?t={cache-version}') !!}
                {!! Theme::js('vendor/adminlte/app.min.js?t={cache-version}') !!}
                {!! Theme::js('vendor/bootstrap-notify/bootstrap-notify.min.js?t={cache-version}') !!}
                {!! Theme::js('vendor/select2/select2.full.min.js?t={cache-version}') !!}
                {!! Theme::js('js/admin/functions.js?t={cache-version}') !!}
                <script src="/js/autocomplete.js" type="application/javascript"></script>

                <script>
                    feather.replace()
                </script>

                @if(Auth::user()->root_admin)
                    <script>
                        $('#logoutButton').on('click', function (event) {
                            event.preventDefault();

                            var that = this;
                            swal({
                                title: 'Deseja deslogar?',
                                type: 'warning',
                                showCancelButton: true,
                                confirmButtonColor: '#d9534f',
                                cancelButtonColor: '#d33',
                                confirmButtonText: 'Deslogar'
                            }, function () {
                                $.ajax({
                                    type: 'POST',
                                    url: '{{ route('auth.logout') }}',
                                    data: {
                                        _token: '{{ csrf_token() }}'
                                    },complete: function () {
                                        window.location.href = '{{route('auth.login')}}';
                                    }
                            });
                        });
                    });
                    </script>
                @endif

                <script>
                    $(function () {
                        $('[data-toggle="tooltip"]').tooltip();
                    })
                </script>
            @show
        </body>
    </head>
</html>
