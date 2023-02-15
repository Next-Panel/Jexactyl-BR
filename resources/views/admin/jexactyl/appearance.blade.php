@extends('layouts.admin')
@include('partials/admin.jexactyl.nav', ['activeTab' => 'appearance'])

@section('title')
    Configurações de Tema
@endsection

@section('content-header')
    <h1>Aparência do Jexactyl <small>Configurar o tema para Jexactyl.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Admin</a></li>
        <li class="active">Jexactyl</li>
    </ol>
@endsection

@section('content')
    @yield('jexactyl::nav')
    <div class="row">
        <div class="col-xs-12">
            <form action="{{ route('admin.jexactyl.appearance') }}" method="POST">
            <div class="box box-info">
                    <div class="box-header with-border">
                        <h3 class="box-title">Configurações gerais <small>Definir configurações gerais de aparência.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Nome do Painel</label>
                                <div>
                                    <input type="text" class="form-control" name="app:name" value="{{ old('app:name', config('app.name')) }}" />
                                    <p class="text-muted"><small>Este é o nome que é usado em todo o painel e em E-mails enviados aos clientes.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Logo do Painel</label>
                                <div>
                                    <input type="text" class="form-control" name="app:logo" value="{{ $logo }}" />
                                    <p class="text-muted"><small>O logotipo que é usado para o front-end do painel.</small></p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="box box-info">
                    <div class="box-header with-border">
                        <h3 class="box-title">Configurações do tema<small>A seleção para o tema de Jexactyl.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Tema do Painel Administrativo</label>
                                <div>
                                    <select name="theme:admin" class="form-control">
                                        <option @if ($admin == 'jexactyl') selected @endif value="jexactyl">Tema padrão</option>
                                        <option @if ($admin == 'dark') selected @endif value="dark">Tema Escuro</option>
                                        <option @if ($admin == 'light') selected @endif value="light">Tema Claro</option>
                                        <option @if ($admin == 'blue') selected @endif value="blue">Tema Azul</option>
                                        <option @if ($admin == 'minecraft') selected @endif value="minecraft">Tema do Minecraft&#8482; </option>
                                    </select>
                                    <p class="text-muted"><small>Determina o tema para a interface do usuário Admin do Jexactyl.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Tipo de Barra lateral</label>
                                <div>
                                    <select name="sidebar:tema" class="form-control">
                                        <option @if ($tema == 'sidebr') selected @endif value="sidebr">Barra Com Texto(Nova)</option>
                                        <option @if ($tema == 'sidejx') selected @endif value="sidejx">Barra Sem texto(Antiga)</option>
                                    </select>
                                    <p class="text-muted"><small>Determina a barra lateral o painel irá usar.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Plano de fundo da aréa do Cliente</label>
                                <div>
                                    <input type="text" class="form-control" name="theme:user:background" value="{{ old('theme:user:background', config('theme.user.background')) }}" />
                                    <p class="text-muted"><small>Se você inserir um URL aqui, as páginas do cliente terão sua imagem como plano de fundo da página.</small></p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                {!! csrf_field() !!}
                <button type="submit" name="_method" value="PATCH" class="btn btn-default pull-right">Salvar mudanças</button>
            </form>
        </div>
    </div>
@endsection
