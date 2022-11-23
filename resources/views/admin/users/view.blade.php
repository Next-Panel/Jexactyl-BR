@extends('layouts.admin')
@include('partials/admin.users.nav', ['activeTab' => 'overview', 'user' => $user])

@section('title')
    Gerenciar Usu&aacute;rio: {{ $user->username }}
@endsection

@section('content-header')
    <h1>{{ $user->name_first }} {{ $user->name_last}}<small>{{ $user->username }}</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li><a href="{{ route('admin.users') }}">Usu&aacute;rios</a></li>
        <li class="active">{{ $user->username }}</li>
    </ol>
@endsection

@section('content')
    @yield('users::nav')
    <div class="row">
        <form action="{{ route('admin.users.view', $user->id) }}" method="post">
            <div class="col-md-6">
                <div class="box box-primary">
                    <div class="box-header with-border">
                        <h3 class="box-title">Identifica&ccedil;&atilde;o</h3>
                    </div>
                    <div class="box-body">
                        <div class="form-group">
                            <label for="email" class="control-label">Endere&ccedil;o Email</label>
                            <div>
                                <input type="email" name="email" value="{{ $user->email }}" class="form-control form-autocomplete-stop">
                            </div>
                        </div>
                        <div class="form-group">
                            <label for="registered" class="control-label">Usu&aacute;rio</label>
                            <div>
                                <input type="text" name="username" value="{{ $user->username }}" class="form-control form-autocomplete-stop">
                            </div>
                        </div>
                        <div class="form-group">
                            <label for="registered" class="control-label">Primeiro nome</label>
                            <div>
                                <input type="text" name="name_first" value="{{ $user->name_first }}" class="form-control form-autocomplete-stop">
                            </div>
                        </div>
                        <div class="form-group">
                            <label for="registered" class="control-label">&Ugrave;ltimo nome</label>
                            <div>
                                <input type="text" name="name_last" value="{{ $user->name_last }}" class="form-control form-autocomplete-stop">
                            </div>
                        </div>
                        <div class="form-group">
                            <label class="control-label">Idioma padr&atilde;o</label>
                            <div>
                                <select name="language" class="form-control">
                                    @foreach($languages as $key => $value)
                                        <option value="{{ $key }}" @if($user->language === $key) selected @endif>{{ $value }}</option>
                                    @endforeach
                                </select>
                                <p class="text-muted"><small>O idioma padr&atilde;o a ser usado ao renderizar o painel para este usu&aacute;rio.</small></p>
                            </div>
                        </div>
                    </div>
                    <div class="box-footer">
                        {!! csrf_field() !!}
                        {!! method_field('PATCH') !!}
                        <input type="submit" value="Atualizar Usu&aacute;rio" class="btn btn-primary btn-sm">
                    </div>
                </div>
            </div>
            <div class="col-md-6">
                <div class="box">
                    <div class="box-header with-border">
                        <h3 class="box-title">Senha</h3>
                    </div>
                    <div class="box-body">
                        <div class="alert alert-success" style="display:none;margin-bottom:10px;" id="gen_pass"></div>
                        <div class="form-group no-margin-bottom">
                            <label for="password" class="control-label">Senha <span class="field-optional"></span></label>
                            <div>
                                <input type="password" id="password" name="password" class="form-control form-autocomplete-stop">
                                <p class="text-muted small">Deixe em branco para manter a mesma senha deste usu&aacute;rio. O usu&aacute;rio n&atilde;o receber&aacute; nenhuma notifica&ccedil;&atilde;o se a senha for alterada.</p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            <div class="col-md-6">
                <div class="box">
                    <div class="box-header with-border">
                        <h3 class="box-title">Permissoes</h3>
                    </div>
                    <div class="box-body">
                        <div class="form-group">
                            <label for="root_admin" class="control-label">Administrador</label>
                            <div>
                                <select name="root_admin" class="form-control">
                                    <option value="0">@lang('strings.no')</option>
                                    <option value="1" {{ $user->root_admin ? 'selected="selected"' : '' }}>@lang('strings.yes')</option>
                                </select>
                                <p class="text-muted"><small>Definir isso como 'Sim' da ao usu&aacute;rio acesso administrativo total.</small></p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </form>
        <div class="col-xs-12">
            <div class="box box-danger">
                <div class="box-header with-border">
                    <h3 class="box-title">Deletar Usu&aacute;rio</h3>
                </div>
                <div class="box-body">
                    <p class="no-margin">N&atilde;o deve haver servidores associados a essa conta para que ela seja exclu&iacute;da.</p>
                </div>
                <div class="box-footer">
                    <form action="{{ route('admin.users.view', $user->id) }}" method="POST">
                        {!! csrf_field() !!}
                        {!! method_field('DELETE') !!}
                        <input id="delete" type="submit" class="btn btn-sm btn-danger pull-right" {{ $user->servers->count() < 1 ?: 'disabled' }} value="Deletar Usu&aacute;rio" />
                    </form>
                </div>
            </div>
        </div>
    </div>
@endsection
