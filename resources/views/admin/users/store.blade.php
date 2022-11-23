@extends('layouts.admin')
@include('partials/admin.users.nav', ['activeTab' => 'storefront', 'user' => $user])

@section('title')
    Detalhes da Loja: {{ $user->username }}
@endsection

@section('content-header')
    <h1>{{ $user->name_first }} {{ $user->name_last}}<small>{{ $user->username }}</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li><a href="{{ route('admin.users') }}">Usu&aacute;rios</a></li>
        <li class="{{ route('admin.users.view', ['user' => $user]) }}">{{ $user->username }}</li>
        <li class="active">Loja</li>
    </ol>
@endsection

@section('content')
    @yield('users::nav')
    <div class="row">
            <div class="col-xs-12">
                <div class="box box-info">
                    <div class="box-header with-border">
                        <h3 class="box-title">Recursos do Usu&aacute;rio</h3>
                    </div>
                    <form action="{{ route('admin.users.store', $user->id) }}" method="POST">
                        <div class="box-body">
                            <div class="row">
                                <div class="form-group col-md-4">
                                    <label for="store_balance" class="control-label">Carteira Total </label>
                                    <input type="text" id="store_balance" value="{{ $user->store_balance }}" name="store_balance" class="form-control form-autocomplete-stop">
                                </div>
                                <div class="form-group col-md-4">
                                    <label for="store_cpu" class="control-label">CPU Total dispon&iacute;vel </label>
                                    <input type="text" id="store_cpu" value="{{ $user->store_cpu }}" name="store_cpu" class="form-control form-autocomplete-stop">
                                </div>
                                <div class="form-group col-md-4">
                                    <label for="store_memory" class="control-label">Memoria(RAM) Total dispon&iacute;vel</label>
                                    <input type="text" id="store_memory" value="{{ $user->store_memory }}" name="store_memory" class="form-control form-autocomplete-stop">
                                </div>
                                <div class="form-group col-md-4">
                                    <label for="store_disk" class="control-label">Disco Total dispon&iacute;vel</label>
                                    <input type="text" id="store_disk" value="{{ $user->store_disk }}" name="store_disk" class="form-control form-autocomplete-stop">
                                </div>
                                <div class="form-group col-md-4">
                                    <label for="store_slots" class="control-label">Slots de Servidores dispon&iacute;vel</label>
                                    <input type="text" id="store_slots" value="{{ $user->store_slots }}" name="store_slots" class="form-control form-autocomplete-stop">
                                </div>
                                <div class="form-group col-md-4">
                                    <label for="store_ports" class="control-label">Portas Total dispon&iacute;vel</label>
                                    <input type="text" id="store_ports" value="{{ $user->store_ports }}" name="store_ports" class="form-control form-autocomplete-stop">
                                </div>
                                <div class="form-group col-md-4">
                                    <label for="store_backups" class="control-label">Backups Totais dispon&iacute;vel</label>
                                    <input type="text" id="store_backups" value="{{ $user->store_backups }}" name="store_backups" class="form-control form-autocomplete-stop">
                                </div>
                                <div class="form-group col-md-4">
                                    <label for="store_databases" class="control-label">Banco de Dados dispon&iacute;veis</label>
                                    <input type="text" id="store_databases" value="{{ $user->store_databases }}" name="store_databases" class="form-control form-autocomplete-stop">
                                </div>
                            </div>
                        </div>
                        <div class="box-footer">
                            {!! csrf_field() !!}
                            <button type="submit" name="_method" value="PATCH" class="btn btn-sm btn-primary pull-right">Salvar Mudan&ccedil;as</button>
                        </div>
                    </form>
                </div>
            </div>
        </div>
    </div>
@endsection
